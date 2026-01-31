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
use std::collections::{HashMap, HashSet};
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
                        // Get credential info for better error logging
                        let credential_info = match get_credential(db_conn.clone(), job_clone.credential_id).await {
                            Ok(cred) => format!("{} ({})", cred.username, cred.identifier),
                            Err(_) => format!("credential_id {}", job_clone.credential_id),
                        };

                        tracing::error!(
                            "IMAP download failed for job {} [Account: {}]: {}",
                            job_id_for_spawn,
                            credential_info,
                            e
                        );

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
        let credential = get_credential(
            db_conn.clone(),
            job.credential_id,
        )
        .await?;

        tracing::info!(
            "Starting IMAP download for job {} - Account: {} ({}), Type: {:?}, Server: {}",
            job.id,
            credential.username,
            credential.identifier,
            credential.credential_type,
            credential.service_name.as_deref().unwrap_or("unknown")
        );

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
                            job.credential_id,
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
        tracing::info!("Starting sync for all jobs");
        let jobs = db::list_download_jobs(self.db_conn.clone(), None, 100).await?;

        for job in jobs {
            // Sync jobs that are completed or paused (but not failed or cancelled)
            if job.status == DownloadJobStatus::Completed || job.status == DownloadJobStatus::Paused {
                // Check if job is already running
                let active_jobs = self.active_jobs.lock().await;
                let is_running = active_jobs.contains_key(&job.id);
                drop(active_jobs);

                if !is_running {
                    tracing::info!("Starting sync for job {}", job.id);
                    if let Err(e) = self.start_job(job.id).await {
                        tracing::error!("Failed to start job {} during sync: {}", job.id, e);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn restore_interrupted_jobs(&self) -> Result<()> {
        // Don't try to restore interrupted jobs - we'll rely on ensure_jobs_for_all_credentials
        // and sync_all_jobs instead. Each job tracks its state (last_synced_uid) so it will
        // continue from where it left off.
        tracing::info!("Skipping interrupted job restoration - relying on auto-creation and sync");
        Ok(())
    }

    pub async fn ensure_jobs_for_all_credentials(&self) -> Result<()> {
        tracing::info!("Ensuring download jobs exist for all credentials");

        // Get all active credentials
        let credentials = crate::database::credentials::list_credentials(
            self.db_conn.clone(),
            false, // only active credentials
        )
        .await?;

        // Get all existing jobs
        let all_jobs = db::list_download_jobs(self.db_conn.clone(), None, 1000).await?;

        // Build a map of credential IDs to their jobs
        let mut jobs_by_credential: std::collections::HashMap<i64, Vec<&shared_types::download::DownloadJob>> =
            std::collections::HashMap::new();
        for job in &all_jobs {
            jobs_by_credential
                .entry(job.credential_id)
                .or_insert_with(Vec::new)
                .push(job);
        }

        for credential in credentials {
            // Only create jobs for IMAP and OAuth credentials (email accounts)
            if credential.credential_type != CredentialType::Imap
                && credential.credential_type != CredentialType::OAuth {
                continue;
            }

            // Check if this credential has any jobs
            if let Some(existing_jobs) = jobs_by_credential.get(&credential.id) {
                // If there are jobs, reset any that are in "running" or "failed" state to "completed"
                // so they'll be picked up by sync_all_jobs
                for job in existing_jobs {
                    if job.status == DownloadJobStatus::Running || job.status == DownloadJobStatus::Failed {
                        tracing::info!(
                            "Resetting job {} (credential {}) from {:?} to completed",
                            job.id,
                            credential.id,
                            job.status
                        );
                        let _ = db::update_job_status(
                            self.db_conn.clone(),
                            job.id,
                            DownloadJobStatus::Completed,
                            None,
                        )
                        .await;
                    }
                }

                tracing::debug!(
                    "Credential {} already has {} job(s), skipping creation",
                    credential.id,
                    existing_jobs.len()
                );
                continue;
            }

            // Create default IMAP download job
            tracing::info!(
                "Creating default download job for credential {} ({})",
                credential.id,
                credential.username
            );

            let default_config = serde_json::json!({
                "folders": [{
                    "name": "INBOX",
                    "total_messages": 0,
                    "downloaded_messages": 0,
                    "failed_messages": 0,
                    "skipped_messages": 0,
                    "last_synced_uid": null,
                    "is_complete": false
                }],
                "sync_strategy": "inbox-only",
                "last_highest_uid": {},
                "fetch_batch_size": 50,
                "max_age_months": 12
            });

            let request = shared_types::download::CreateDownloadJobRequest {
                credential_id: credential.id,
                source_type: SourceType::Imap,
                source_config: default_config,
            };

            match db::insert_download_job(self.db_conn.clone(), &request).await {
                Ok(job) => {
                    tracing::info!(
                        "Created download job {} for credential {}",
                        job.id,
                        credential.id
                    );

                    // Start the job immediately
                    if let Err(e) = self.start_job(job.id).await {
                        tracing::error!(
                            "Failed to start auto-created job {}: {}",
                            job.id,
                            e
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create job for credential {}: {}",
                        credential.id,
                        e
                    );
                }
            }
        }

        Ok(())
    }
}
