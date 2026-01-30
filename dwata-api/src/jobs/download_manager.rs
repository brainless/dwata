use crate::database::downloads as db;
use crate::database::credentials::get_credential;
use crate::database::AsyncDbConnection;
use crate::database::emails as emails_db;
use crate::helpers::keyring_service::KeyringService;
use crate::integrations::real_imap_client::RealImapClient;
use crate::helpers::imap_oauth::get_access_token_for_imap;
use anyhow::Result;
use shared_types::download::{DownloadJob, DownloadJobStatus, ImapDownloadState, SourceType};
use shared_types::credential::CredentialType;
use shared_types::email::EmailAddress;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::sync::Mutex;

pub struct DownloadManager {
    db_conn: AsyncDbConnection,
    active_jobs: Arc<Mutex<HashMap<i64, JoinHandle<()>>>>,
    token_cache: Arc<crate::helpers::token_cache::TokenCache>,
    oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
}

impl DownloadManager {
    pub fn new(
        db_conn: AsyncDbConnection,
        token_cache: Arc<crate::helpers::token_cache::TokenCache>,
        oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
    ) -> Self {
        Self {
            db_conn,
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
            token_cache,
            oauth_client,
        }
    }

    pub async fn start_job(&self, job_id: i64) -> Result<()> {
        let job = db::get_download_job(self.db_conn.clone(), job_id).await?;

        let active_jobs = self.active_jobs.lock().await;
        if active_jobs.contains_key(&job_id) {
            drop(active_jobs);
            return Err(anyhow::anyhow!("Job already running"));
        }
        drop(active_jobs);

        db::update_job_status(
            self.db_conn.clone(),
            job_id,
            DownloadJobStatus::Running,
            None,
        )
        .await?;

        let db_conn = self.db_conn.clone();
        let job_id_for_spawn = job_id;
        let token_cache = self.token_cache.clone();
        let oauth_client = self.oauth_client.clone();
        let job_clone = job.clone();

        let handle = tokio::spawn(async move {
            match job_clone.source_type {
                SourceType::Imap => {
                    if let Err(e) = Self::run_imap_download(db_conn.clone(), &job_clone, token_cache, oauth_client).await {
                        tracing::error!("IMAP download failed for job {}: {}", job_id_for_spawn, e);
                        let _ = db::update_job_status(
                            db_conn,
                            job_id_for_spawn,
                            DownloadJobStatus::Failed,
                            Some(e.to_string()),
                        )
                        .await;
                    }
                }
                _ => {
                    tracing::warn!("Unsupported source type: {:?}", job_clone.source_type);
                }
            }
        });

        let mut active_jobs = self.active_jobs.lock().await;
        active_jobs.insert(job_id, handle);
        drop(active_jobs);

        Ok(())
    }

    pub async fn pause_job(&self, job_id: i64) -> Result<()> {
        let mut active_jobs = self.active_jobs.lock().await;
        let handle = active_jobs.remove(&job_id);
        drop(active_jobs);

        if let Some(handle) = handle {
            handle.abort();
        }

        db::update_job_status(
            self.db_conn.clone(),
            job_id,
            DownloadJobStatus::Paused,
            None,
        )
        .await?;

        Ok(())
    }

    async fn run_imap_download(
        db_conn: Arc<Mutex<duckdb::Connection>>,
        job: &DownloadJob,
        token_cache: Arc<crate::helpers::token_cache::TokenCache>,
        oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
    ) -> Result<()> {
        tracing::info!("Starting IMAP download for job {}", job.id);

        let credential = get_credential(
            db_conn.clone(),
            job.credential_id,
        )
        .await?;

        let mut imap_client = if credential.credential_type == CredentialType::OAuth {
            let access_token = get_access_token_for_imap(
                credential.id,
                &credential,
                &token_cache,
                &oauth_client,
            )
            .await?;

            RealImapClient::connect_with_oauth(
                &credential.service_name.unwrap_or("imap.gmail.com".to_string()),
                credential.port.unwrap_or(993) as u16,
                &credential.username,
                &access_token,
            )
            .await?
        } else {
            let password = KeyringService::get_password(
                &credential.credential_type,
                &credential.identifier,
                &credential.username,
            )?;

            RealImapClient::connect_with_password(
                &credential.service_name.unwrap_or_default(),
                credential.port.unwrap_or(993) as u16,
                &credential.username,
                &password,
            )
            .await?
        };

        let state: ImapDownloadState = serde_json::from_value(job.source_state.clone())?;
        let max_age_months = state.max_age_months.or(Some(12));

        for folder in &state.folders {
            tracing::info!("Processing folder: {}", folder.name);

            let mailbox_status = imap_client.mailbox_status(&folder.name)?;

            if mailbox_status != folder.total_messages {
                tracing::info!(
                    "Folder {} message count changed: {} -> {}",
                    folder.name,
                    folder.total_messages,
                    mailbox_status
                );
            }

            let resume_uid = folder.last_synced_uid;
            let uids = imap_client
                .search_emails(
                    &folder.name,
                    resume_uid,
                    max_age_months,
                    Some(state.fetch_batch_size)
                )?;

            tracing::info!("Found {} emails to download in {}", uids.len(), folder.name);

            for uid in uids {
                match imap_client.fetch_email(&folder.name, uid) {
                    Ok(parsed_email) => {
                        let download_item_id = db::insert_download_item(
                            db_conn.clone(),
                            job.id,
                            &uid.to_string(),
                            Some(&folder.name),
                            "email",
                            "completed",
                            parsed_email.size_bytes.map(|s| s as i64),
                            Some("message/rfc822"),
                            None,
                        ).await?;

                        let to_addresses: Vec<EmailAddress> = parsed_email.to_addresses
                            .iter()
                            .filter_map(|(addr, name)| {
                                addr.as_ref().map(|a| EmailAddress {
                                    email: a.clone(),
                                    name: name.clone(),
                                })
                            })
                            .collect();

                        let email_id = emails_db::insert_email(
                            db_conn.clone(),
                            Some(download_item_id),
                            parsed_email.uid,
                            &folder.name,
                            parsed_email.message_id.as_deref(),
                            parsed_email.subject.as_deref(),
                            &parsed_email.from_address.unwrap_or_default(),
                            parsed_email.from_name.as_deref(),
                            &to_addresses,
                            &[],
                            &[],
                            parsed_email.reply_to.as_deref(),
                            parsed_email.date_sent,
                            parsed_email.date_received,
                            parsed_email.body_text.as_deref(),
                            parsed_email.body_html.as_deref(),
                            parsed_email.is_read,
                            parsed_email.is_flagged,
                            parsed_email.is_draft,
                            parsed_email.is_answered,
                            parsed_email.has_attachments,
                            parsed_email.attachment_count,
                            parsed_email.size_bytes,
                            &parsed_email.labels,
                        ).await?;

                        tracing::info!("Downloaded and stored email UID {} (id: {})", uid, email_id);

                        db::update_job_progress(
                            db_conn.clone(),
                            job.id,
                            None,
                            Some(1),
                            None,
                            None,
                            parsed_email.size_bytes.map(|s| s as u64),
                        )
                        .await?;
                    }
                    Err(e) => {
                        tracing::error!("Failed to download email UID {}: {}", uid, e);

                        db::update_job_progress(
                            db_conn.clone(),
                            job.id,
                            None,
                            None,
                            Some(1),
                            None,
                            None,
                        )
                        .await?;
                    }
                }
            }
        }

        db::update_job_status(
            db_conn,
            job.id,
            DownloadJobStatus::Completed,
            None,
        )
        .await?;

        tracing::info!("IMAP download completed for job {}", job.id);
        Ok(())
    }

    pub async fn sync_all_jobs(&self) -> Result<()> {
        let jobs = db::list_download_jobs(self.db_conn.clone(), None, 100).await?;

        for job in jobs {
            if job.status == DownloadJobStatus::Running || job.status == DownloadJobStatus::Completed {
            }
        }

        Ok(())
    }

    pub async fn restore_interrupted_jobs(&self) -> Result<()> {
        let interrupted_jobs = db::list_download_jobs(
            self.db_conn.clone(),
            Some("running"),
            100,
        )
        .await?;

        for job in interrupted_jobs {
            tracing::info!("Resuming interrupted job: {}", job.id);
            self.start_job(job.id).await?;
        }

        Ok(())
    }
}
