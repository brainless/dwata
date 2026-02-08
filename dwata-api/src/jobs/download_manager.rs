use crate::database::downloads as db;
use crate::database::credentials::get_credential;
use crate::database::emails;
use crate::database::folders;
use crate::database::AsyncDbConnection;
use crate::helpers::keyring_service::KeyringService;
use crate::integrations::real_imap_client::RealImapClient;
use crate::helpers::imap_oauth::get_access_token_for_imap;
use anyhow::Result;
use shared_types::download::{DownloadJob, DownloadJobStatus, ImapDownloadState, SourceType, JobType};
use shared_types::credential::CredentialType;
use shared_types::email::EmailAddress;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;
use tokio::sync::Mutex;

pub struct DownloadManager {
    db_conn: AsyncDbConnection,
    active_jobs: Arc<Mutex<HashMap<i64, JoinHandle<()>>>>,
    token_cache: Arc<crate::helpers::token_cache::TokenCache>,
    oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
    keyring_service: Arc<KeyringService>,
    shutdown_flag: Arc<AtomicBool>,
}

impl DownloadManager {
    pub fn new(
        db_conn: AsyncDbConnection,
        token_cache: Arc<crate::helpers::token_cache::TokenCache>,
        oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
        keyring_service: Arc<KeyringService>,
    ) -> Self {
        Self {
            db_conn,
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
            token_cache,
            oauth_client,
            keyring_service,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_db_connection(&self) -> AsyncDbConnection {
        self.db_conn.clone()
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    pub async fn start_job(&self, job_id: i64) -> Result<()> {
        if self.shutdown_flag.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Download manager is shutting down"));
        }

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
        let keyring_service = self.keyring_service.clone();
        let job_clone = job.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let active_jobs_cleanup = self.active_jobs.clone();

        let handle = tokio::spawn(async move {
            match job_clone.source_type {
                SourceType::Imap => {
                    if let Err(e) = Self::run_imap_download(
                        db_conn.clone(),
                        &job_clone,
                        token_cache,
                        oauth_client,
                        keyring_service,
                        shutdown_flag.clone(),
                    ).await {
                        // Get credential info for better error logging
                        let credential_info = match get_credential(db_conn.clone(), job_clone.credential_id).await {
                            Ok(cred) => format!("{} ({})", cred.username, cred.identifier),
                            Err(_) => format!("credential_id {}", job_clone.credential_id),
                        };

                        let error_str = e.to_string();

                        // Check if this is a transient error (network, timeout, request failed)
                        let is_transient = error_str.contains("Request failed")
                            || error_str.contains("timeout")
                            || error_str.contains("connection")
                            || error_str.contains("network");

                        if shutdown_flag.load(Ordering::SeqCst) {
                            let _ = db::update_job_status(
                                db_conn,
                                job_id_for_spawn,
                                DownloadJobStatus::Cancelled,
                                Some("Server shutdown".to_string()),
                            )
                            .await;
                        } else if is_transient {
                            tracing::warn!(
                                "IMAP download encountered transient error for job {} [Account: {}]: {}. Will retry on next sync.",
                                job_id_for_spawn,
                                credential_info,
                                e
                            );

                            // Set back to Completed so it will retry on next sync
                            let _ = db::update_job_status(
                                db_conn,
                                job_id_for_spawn,
                                DownloadJobStatus::Completed,
                                None,
                            )
                            .await;
                        } else {
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
                                Some(error_str),
                            )
                            .await;
                        }
                    }
                }
                _ => {
                    tracing::warn!("Unsupported source type: {:?}", job_clone.source_type);
                }
            }

            let mut active_jobs = active_jobs_cleanup.lock().await;
            active_jobs.remove(&job_id_for_spawn);
        });

        let mut active_jobs = self.active_jobs.lock().await;
        active_jobs.insert(job_id, handle);
        drop(active_jobs);

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown_flag.store(true, Ordering::SeqCst);

        let mut active_jobs = self.active_jobs.lock().await;
        let handles = std::mem::take(&mut *active_jobs);
        drop(active_jobs);

        for (job_id, handle) in handles {
            handle.abort();
            let _ = db::update_job_status(
                self.db_conn.clone(),
                job_id,
                DownloadJobStatus::Cancelled,
                Some("Server shutdown".to_string()),
            )
            .await;
        }

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
        db_conn: AsyncDbConnection,
        job: &DownloadJob,
        token_cache: Arc<crate::helpers::token_cache::TokenCache>,
        oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
        keyring_service: Arc<KeyringService>,
        shutdown_flag: Arc<AtomicBool>,
    ) -> Result<()> {
        let handle = tokio::runtime::Handle::current();
        let job = job.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        std::thread::spawn(move || {
            let result = handle.block_on(Self::run_imap_download_async(
                db_conn,
                job,
                token_cache,
                oauth_client,
                keyring_service,
                shutdown_flag,
            ));
            let _ = tx.send(result);
        });

        rx.await
            .map_err(|e| anyhow::anyhow!("IMAP worker thread stopped: {}", e))?
    }

    async fn run_imap_download_async(
        db_conn: AsyncDbConnection,
        job: DownloadJob,
        token_cache: Arc<crate::helpers::token_cache::TokenCache>,
        oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
        keyring_service: Arc<KeyringService>,
        shutdown_flag: Arc<AtomicBool>,
    ) -> Result<()> {
        if shutdown_flag.load(Ordering::SeqCst) {
            db::update_job_status(
                db_conn.clone(),
                job.id,
                DownloadJobStatus::Cancelled,
                Some("Server shutdown".to_string()),
            )
            .await?;
            return Ok(());
        }

        let credential = get_credential(
            db_conn.clone(),
            job.credential_id,
        )
        .await?;

        tracing::info!(
            "Starting IMAP download for job {} - Account: {} ({}), Type: {:?}, Server: {}, Job Type: {:?}",
            job.id,
            credential.username,
            credential.identifier,
            credential.credential_type,
            credential.service_name.as_deref().unwrap_or("unknown"),
            job.job_type
        );

        let mut imap_client = if credential.credential_type == CredentialType::OAuth {
            let access_token = get_access_token_for_imap(
                credential.id,
                &credential,
                &token_cache,
                &oauth_client,
                &keyring_service,
            )
            .await?;

            RealImapClient::connect_with_oauth(
                &credential.service_name.unwrap_or("imap.gmail.com".to_string()),
                credential.port.unwrap_or(993) as u16,
                &credential.username,
                &access_token,
            )?
        } else {
            let password = keyring_service
                .get_password(
                    &credential.credential_type,
                    &credential.identifier,
                    &credential.username,
                )
                .await?;

            RealImapClient::connect_with_password(
                &credential.service_name.unwrap_or_default(),
                credential.port.unwrap_or(993) as u16,
                &credential.username,
                &password,
            )?
        };

        let state: ImapDownloadState = serde_json::from_value(job.source_state.clone())?;
        let max_age_months = state.max_age_months.or(Some(12));

        // Discover and sync folders for this credential
        tracing::info!("Discovering folders for credential {}", job.credential_id);
        let folders = imap_client.list_folders_with_metadata()?;

        // Store folder metadata in database
        for folder in &folders {
            let mailbox_status = match imap_client.mailbox_status(&folder.imap_path) {
                Ok(status) => status,
                Err(e) => {
                    tracing::warn!("Failed to get status for folder '{}': {}. Skipping.", folder.imap_path, e);
                    continue;
                }
            };

            let _folder_id = folders::upsert_folder_from_imap(
                db_conn.clone(),
                job.credential_id,
                &folder.name,
                &folder.imap_path,
                folder.is_selectable,
                folder.is_subscribed,
                None,
                mailbox_status,
            ).await?;

            // TODO: Handle UIDVALIDITY changes
            // If folder.uidvalidity changes, we should reset last_synced_uid to 0
            // and re-sync all emails in this folder
        }

        let db_folders = folders::list_folders_for_credential(db_conn.clone(), job.credential_id).await?;

        tracing::info!("Found {} folders for credential {}", db_folders.len(), job.credential_id);

        for db_folder in db_folders {
            if shutdown_flag.load(Ordering::SeqCst) {
                db::update_job_status(
                    db_conn.clone(),
                    job.id,
                    DownloadJobStatus::Cancelled,
                    Some("Server shutdown".to_string()),
                )
                .await?;
                return Ok(());
            }

            if !db_folder.is_selectable {
                tracing::debug!("Skipping non-selectable folder: {}", db_folder.imap_path);
                continue;
            }

            tracing::info!("Processing folder: {}", db_folder.imap_path);

            let resume_uid = db_folder.last_synced_uid;
            let uids = match job.job_type {
                JobType::RecentSync => {
                    // Download new emails (UID > last_synced_uid)
                    match imap_client.search_emails(
                        &db_folder.imap_path,
                        resume_uid,
                        max_age_months,
                        Some(state.fetch_batch_size)
                    ) {
                        Ok(uids) => uids,
                        Err(e) => {
                            tracing::warn!("Failed to search emails in folder '{}': {}. Skipping.", db_folder.imap_path, e);
                            continue;
                        }
                    }
                }
                JobType::HistoricalBackfill => {
                    // Download historical emails (oldest first, up to 100 per folder)
                    // Try reverse chronological (oldest UIDs first)
                    let all_uids = match imap_client.search_emails(
                        &db_folder.imap_path,
                        None,
                        None,
                        None,
                    ) {
                        Ok(uids) => uids,
                        Err(e) => {
                            tracing::warn!("Failed to search emails in folder '{}': {}. Skipping.", db_folder.imap_path, e);
                            continue;
                        }
                    };

                    // Filter for emails we haven't synced yet (UID <= last_synced_uid or last_synced_uid is None)
                    let historical_uids: Vec<u32> = all_uids
                        .into_iter()
                        .filter(|uid| resume_uid.map_or(true, |last| uid <= &last))
                        .take(100)
                        .collect();

                    historical_uids
                }
            };

            let uids = emails::filter_new_uids(
                db_conn.clone(),
                job.credential_id,
                db_folder.id,
                &uids,
            ).await?;

            tracing::info!("Found {} new emails to download in {}", uids.len(), db_folder.imap_path);

            let mut highest_uid = db_folder.last_synced_uid;

            for uid in uids {
                if shutdown_flag.load(Ordering::SeqCst) {
                    db::update_job_status(
                        db_conn.clone(),
                        job.id,
                        DownloadJobStatus::Cancelled,
                        Some("Server shutdown".to_string()),
                    )
                    .await?;
                    return Ok(());
                }

                match imap_client.fetch_email(&db_folder.imap_path, uid) {
                    Ok(parsed_email) => {
                        let to_addresses: Vec<EmailAddress> = parsed_email.to_addresses
                            .iter()
                            .filter_map(|(addr, name)| {
                                addr.as_ref().map(|a| EmailAddress {
                                    email: a.clone(),
                                    name: name.clone(),
                                })
                            })
                            .collect();

                        // Use transactional insert to ensure atomicity
                        match db::insert_email_download_transactional(
                            db_conn.clone(),
                            job.id,
                            job.credential_id,
                            parsed_email.uid,
                            db_folder.id,
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
                        ).await {
                            Ok((_download_item_id, email_id)) => {
                                tracing::info!("Downloaded and stored email UID {} (id: {}) in transaction", uid, email_id);
                                // Track highest UID for updating folder sync state
                                highest_uid = Some(highest_uid.map_or(uid, |last| last.max(uid)));
                            }
                            Err(e) => {
                                tracing::error!("Failed to store email UID {} in transaction: {}", uid, e);
                                // Update failed count
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

            // Update folder sync state with highest UID processed
            if let Some(uid) = highest_uid {
                folders::update_folder_sync_state(
                    db_conn.clone(),
                    db_folder.id,
                    uid,
                    uid,
                ).await?;
                tracing::info!("Updated folder {} sync state to UID {}", db_folder.imap_path, uid);
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
        tracing::info!("Starting sync for all recent-sync jobs");
        let jobs = db::list_download_jobs(self.db_conn.clone(), None, 100).await?;

        for job in jobs {
            // Only sync recent-sync jobs
            if !matches!(job.job_type, JobType::RecentSync) {
                continue;
            }

            // Sync jobs that are completed or paused (but not failed or cancelled)
            if job.status == DownloadJobStatus::Completed || job.status == DownloadJobStatus::Paused {
                // Check if job is already running
                let active_jobs = self.active_jobs.lock().await;
                let is_running = active_jobs.contains_key(&job.id);
                drop(active_jobs);

                if !is_running {
                    tracing::info!("Starting sync for recent-sync job {}", job.id);
                    if let Err(e) = self.start_job(job.id).await {
                        tracing::error!("Failed to start job {} during sync: {}", job.id, e);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn start_historical_backfill(&self, credential_id: i64) -> Result<()> {
        tracing::info!("Starting historical backfill for credential {}", credential_id);

        let jobs = db::list_download_jobs(self.db_conn.clone(), None, 100).await?;

        for job in jobs {
            if job.credential_id == credential_id && matches!(job.job_type, JobType::HistoricalBackfill) {
                // Check if job is already running
                let active_jobs = self.active_jobs.lock().await;
                let is_running = active_jobs.contains_key(&job.id);
                drop(active_jobs);

                if !is_running {
                    tracing::info!("Starting historical backfill job {}", job.id);
                    return self.start_job(job.id).await;
                }
            }
        }

        Err(anyhow::anyhow!("No historical backfill job found for credential {}", credential_id))
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
                // Check if we have both job types
                let has_recent_sync = existing_jobs.iter().any(|j| matches!(j.job_type, JobType::RecentSync));
                let has_historical = existing_jobs.iter().any(|j| matches!(j.job_type, JobType::HistoricalBackfill));

                if has_recent_sync && has_historical {
                    // Reset any that are in "running" or "failed" state to "completed"
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
                    continue;
                }
            }

            // Create download jobs for this credential
            tracing::info!(
                "Creating download jobs for credential {} ({})",
                credential.id,
                credential.username
            );

            let default_config = serde_json::json!({
                "sync_strategy": "full-sync",
                "last_highest_uid": {},
                "fetch_batch_size": 50,
                "max_age_months": 12
            });

            // Create RecentSync job
            let request = shared_types::download::CreateDownloadJobRequest {
                credential_id: credential.id,
                source_type: SourceType::Imap,
                source_config: default_config.clone(),
            };

            match db::insert_download_job(self.db_conn.clone(), &request, JobType::RecentSync).await {
                Ok(job) => {
                    tracing::info!(
                        "Created recent-sync job {} for credential {}",
                        job.id,
                        credential.id
                    );
                    // Start recent-sync job immediately
                    if let Err(e) = self.start_job(job.id).await {
                        tracing::error!(
                            "Failed to start recent-sync job {}: {}",
                            job.id,
                            e
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create recent-sync job for credential {}: {}",
                        credential.id,
                        e
                    );
                }
            }

            // Create HistoricalBackfill job
            match db::insert_download_job(self.db_conn.clone(), &request, JobType::HistoricalBackfill).await {
                Ok(job) => {
                    tracing::info!(
                        "Created historical-backfill job {} for credential {}",
                        job.id,
                        credential.id
                    );
                    // Don't start historical backfill immediately - it will be triggered on-demand
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create historical-backfill job for credential {}: {}",
                        credential.id,
                        e
                    );
                }
            }
        }

        Ok(())
    }
}
