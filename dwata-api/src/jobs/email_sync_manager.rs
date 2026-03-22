use crate::database::credentials::get_credential;
use crate::database::emails;
use crate::database::folders;
use crate::database::AsyncDbConnection;
use crate::helpers::imap_oauth::get_access_token_for_imap;
use crate::helpers::keyring_service::KeyringService;
use crate::integrations::real_imap_client::RealImapClient;
use anyhow::Result;
use shared_types::credential::CredentialType;
use shared_types::download::EmailSyncDirection;
use shared_types::email::EmailAddress;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;

/// Maximum number of IMAP folders synced in parallel per credential
const FOLDER_PARALLELISM: usize = 4;

pub struct EmailSyncManager {
    db_conn: AsyncDbConnection,
    token_cache: Arc<crate::helpers::token_cache::TokenCache>,
    oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
    keyring_service: Arc<KeyringService>,
    /// Active sync task handles, keyed by credential_id
    active_syncs: Arc<Mutex<HashMap<i64, tokio::task::JoinHandle<()>>>>,
    shutdown_flag: Arc<AtomicBool>,
    credential_semaphores: Arc<Mutex<HashMap<i64, Arc<Semaphore>>>>,
}

impl EmailSyncManager {
    pub fn new(
        db_conn: AsyncDbConnection,
        token_cache: Arc<crate::helpers::token_cache::TokenCache>,
        oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
        keyring_service: Arc<KeyringService>,
    ) -> Self {
        Self {
            db_conn,
            token_cache,
            oauth_client,
            keyring_service,
            active_syncs: Arc::new(Mutex::new(HashMap::new())),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            credential_semaphores: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_db_connection(&self) -> AsyncDbConnection {
        self.db_conn.clone()
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Spawn a background sync task for one credential. Returns immediately.
    /// If a sync is already running for this credential it is skipped.
    pub async fn sync_credential(
        &self,
        credential_id: i64,
        direction: EmailSyncDirection,
    ) -> Result<()> {
        if self.shutdown_flag.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("EmailSyncManager is shutting down"));
        }

        {
            let active = self.active_syncs.lock().await;
            if active.contains_key(&credential_id) {
                tracing::debug!(
                    credential_id,
                    "sync already running, skipping duplicate request"
                );
                return Ok(());
            }
        }

        let db_conn = self.db_conn.clone();
        let token_cache = self.token_cache.clone();
        let oauth_client = self.oauth_client.clone();
        let keyring_service = self.keyring_service.clone();
        let credential_semaphores = self.credential_semaphores.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let active_syncs_cleanup = self.active_syncs.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = Self::run_imap_sync(
                db_conn.clone(),
                credential_id,
                &direction,
                token_cache,
                oauth_client,
                keyring_service,
                credential_semaphores,
                shutdown_flag.clone(),
            )
            .await
            {
                let error_str = e.to_string();
                let is_transient = error_str.contains("Request failed")
                    || error_str.contains("timeout")
                    || error_str.contains("connection")
                    || error_str.contains("network");

                if shutdown_flag.load(Ordering::SeqCst) {
                    tracing::info!(credential_id, "sync cancelled (server shutdown)");
                } else if is_transient {
                    tracing::warn!(
                        credential_id,
                        error = %e,
                        "transient IMAP error — will retry on next sync"
                    );
                } else {
                    tracing::error!(credential_id, error = %e, "IMAP sync failed");
                }
            }

            active_syncs_cleanup.lock().await.remove(&credential_id);
        });

        self.active_syncs.lock().await.insert(credential_id, handle);
        Ok(())
    }

    /// Trigger recent sync for all credentials.
    pub async fn sync_all_recent(&self) -> Result<()> {
        let credentials = self.imap_credentials().await?;
        tracing::info!(
            count = credentials.len(),
            "starting recent sync for all accounts"
        );
        for cred_id in credentials {
            if let Err(e) = self
                .sync_credential(cred_id, EmailSyncDirection::Recent)
                .await
            {
                tracing::warn!(credential_id = cred_id, error = %e, "failed to start recent sync");
            }
        }
        Ok(())
    }

    /// Trigger backfill for all credentials.
    pub async fn sync_all_backfill(&self) -> Result<()> {
        let credentials = self.imap_credentials().await?;
        tracing::info!(
            count = credentials.len(),
            "starting backfill for all accounts"
        );
        for cred_id in credentials {
            if let Err(e) = self
                .sync_credential(cred_id, EmailSyncDirection::Backfill)
                .await
            {
                tracing::warn!(
                    credential_id = cred_id,
                    error = %e,
                    "failed to start backfill"
                );
            }
        }
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        let handles: Vec<_> = self.active_syncs.lock().await.drain().collect();
        for (credential_id, handle) in handles {
            tracing::info!(credential_id, "aborting sync task for shutdown");
            handle.abort();
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    async fn imap_credentials(&self) -> Result<Vec<i64>> {
        let all_credentials =
            crate::database::credentials::list_credentials(self.db_conn.clone(), false).await?;

        Ok(all_credentials
            .into_iter()
            .filter(|c| {
                c.credential_type == CredentialType::Imap
                    || c.credential_type == CredentialType::OAuth
            })
            .map(|c| c.id)
            .collect())
    }

    // -------------------------------------------------------------------------
    // Core sync logic — public so CLI can call it directly
    // -------------------------------------------------------------------------

    /// Run a full IMAP sync for one credential synchronously.
    /// This is the shared implementation used by both the background task
    /// (via `sync_credential`) and the CLI binary.
    pub async fn run_imap_sync(
        db_conn: AsyncDbConnection,
        credential_id: i64,
        direction: &EmailSyncDirection,
        token_cache: Arc<crate::helpers::token_cache::TokenCache>,
        oauth_client: Arc<crate::helpers::google_oauth::GoogleOAuthClient>,
        keyring_service: Arc<KeyringService>,
        credential_semaphores: Arc<Mutex<HashMap<i64, Arc<Semaphore>>>>,
        shutdown_flag: Arc<AtomicBool>,
    ) -> Result<()> {
        if shutdown_flag.load(Ordering::SeqCst) {
            return Ok(());
        }

        let credential = get_credential(db_conn.clone(), credential_id).await?;

        tracing::info!(
            credential_id,
            email = %credential.username,
            identifier = %credential.identifier,
            direction = ?direction,
            "starting IMAP sync"
        );

        let server = credential
            .service_name
            .clone()
            .unwrap_or_else(|| "imap.gmail.com".to_string());
        let port = credential.port.unwrap_or(993) as u16;
        let username = credential.username.clone();

        // Resolve auth
        enum Auth {
            OAuth(String),
            Password(String),
        }

        let auth = if credential.credential_type == CredentialType::OAuth {
            let token = get_access_token_for_imap(
                credential.id,
                &credential,
                &token_cache,
                &oauth_client,
                &keyring_service,
            )
            .await?;
            Auth::OAuth(token)
        } else {
            let pw = keyring_service
                .get_password(
                    &credential.credential_type,
                    &credential.identifier,
                    &credential.username,
                )
                .await?;
            Auth::Password(pw)
        };

        // Discover folders and upsert them to DB
        let mut imap_client = match &auth {
            Auth::OAuth(t) => RealImapClient::connect_with_oauth(&server, port, &username, t)?,
            Auth::Password(p) => {
                RealImapClient::connect_with_password(&server, port, &username, p)?
            }
        };

        tracing::info!(credential_id, "discovering IMAP folders");
        let folders = imap_client.list_folders_with_metadata()?;

        for folder in &folders {
            match imap_client.mailbox_status(&folder.imap_path) {
                Ok(status) => {
                    if let Err(e) = folders::upsert_folder_from_imap(
                        db_conn.clone(),
                        credential_id,
                        &folder.name,
                        &folder.imap_path,
                        folder.is_selectable,
                        folder.is_subscribed,
                        None,
                        status,
                    )
                    .await
                    {
                        tracing::warn!(folder = %folder.imap_path, error = %e, "failed to upsert folder");
                    }
                }
                Err(e) => {
                    tracing::warn!(folder = %folder.imap_path, error = %e, "failed to get mailbox status");
                }
            }
        }

        let db_folders =
            folders::list_folders_for_credential(db_conn.clone(), credential_id).await?;
        tracing::info!(
            credential_id,
            folder_count = db_folders.len(),
            "syncing folders"
        );

        // Per-credential semaphore to cap parallelism
        let semaphore = {
            let mut sems = credential_semaphores.lock().await;
            sems.entry(credential_id)
                .or_insert_with(|| Arc::new(Semaphore::new(FOLDER_PARALLELISM)))
                .clone()
        };

        let auth = Arc::new(auth);
        let direction = Arc::new(direction.clone());
        let mut join_set = JoinSet::new();

        for db_folder in db_folders {
            if !db_folder.is_selectable {
                continue;
            }

            let db_conn = db_conn.clone();
            let server = server.clone();
            let username = username.clone();
            let auth = auth.clone();
            let semaphore = semaphore.clone();
            let shutdown_flag = shutdown_flag.clone();
            let direction = direction.clone();

            join_set.spawn(async move {
                let _permit = semaphore.acquire().await?;
                if shutdown_flag.load(Ordering::SeqCst) {
                    return Ok(());
                }

                let handle = tokio::runtime::Handle::current();
                tokio::task::spawn_blocking(move || {
                    if shutdown_flag.load(Ordering::SeqCst) {
                        return Ok(());
                    }

                    let mut client = match auth.as_ref() {
                        Auth::OAuth(t) => {
                            RealImapClient::connect_with_oauth(&server, port, &username, t)?
                        }
                        Auth::Password(p) => {
                            RealImapClient::connect_with_password(&server, port, &username, p)?
                        }
                    };

                    tracing::info!(folder = %db_folder.imap_path, "processing folder");

                    let mailbox_meta = match client.get_mailbox_metadata(&db_folder.imap_path) {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!(
                                folder = %db_folder.imap_path,
                                error = %e,
                                "failed to get mailbox metadata, skipping"
                            );
                            return Ok(());
                        }
                    };

                    if let Some(stored) = db_folder.uidvalidity {
                        if stored != mailbox_meta.uidvalidity {
                            tracing::warn!(
                                folder = %db_folder.imap_path,
                                old_uidvalidity = stored,
                                new_uidvalidity = mailbox_meta.uidvalidity,
                                "UIDVALIDITY changed — folder may need full re-sync"
                            );
                        }
                    }

                    // Determine which UIDs to fetch based on direction
                    const BATCH_SIZE: usize = 100;

                    let uids: Vec<u32> = match direction.as_ref() {
                        EmailSyncDirection::Recent => {
                            let resume_uid = db_folder.last_synced_uid;
                            tracing::info!(
                                folder = %db_folder.imap_path,
                                resume_uid = ?resume_uid,
                                "recent sync"
                            );
                            match client.search_emails(
                                &db_folder.imap_path,
                                resume_uid,
                                None,
                                None,
                                Some(BATCH_SIZE),
                            ) {
                                Ok(u) => u,
                                Err(e) => {
                                    tracing::warn!(
                                        folder = %db_folder.imap_path,
                                        error = %e,
                                        "search failed, skipping"
                                    );
                                    return Ok(());
                                }
                            }
                        }
                        EmailSyncDirection::Backfill => {
                            let oldest_uid = db_folder.oldest_synced_uid.or_else(|| {
                                handle
                                    .block_on(emails::get_oldest_uid_for_folder(
                                        db_conn.clone(),
                                        credential_id,
                                        db_folder.id,
                                    ))
                                    .ok()
                                    .flatten()
                            });

                            tracing::info!(
                                folder = %db_folder.imap_path,
                                oldest_uid = ?oldest_uid,
                                "backfill"
                            );

                            if oldest_uid == Some(1) {
                                tracing::info!(
                                    folder = %db_folder.imap_path,
                                    "already at oldest UID, nothing to backfill"
                                );
                                return Ok(());
                            }

                            let before_uid = oldest_uid;
                            let mut all_uids = match client.search_emails(
                                &db_folder.imap_path,
                                None,
                                before_uid,
                                None,
                                None,
                            ) {
                                Ok(u) => u,
                                Err(e) => {
                                    tracing::warn!(
                                        folder = %db_folder.imap_path,
                                        error = %e,
                                        "search failed, skipping"
                                    );
                                    return Ok(());
                                }
                            };
                            // Take newest-first from the historical range
                            all_uids.sort_unstable_by(|a, b| b.cmp(a));
                            all_uids.truncate(BATCH_SIZE);
                            all_uids
                        }
                    };

                    // Filter UIDs we already have
                    let uids = handle.block_on(emails::filter_new_uids(
                        db_conn.clone(),
                        credential_id,
                        db_folder.id,
                        &uids,
                    ))?;

                    tracing::info!(
                        folder = %db_folder.imap_path,
                        new_email_count = uids.len(),
                        "fetching new emails"
                    );

                    let mut highest_uid: Option<u32> = None;
                    let mut lowest_uid: Option<u32> = None;

                    for uid in &uids {
                        if shutdown_flag.load(Ordering::SeqCst) {
                            return Ok(());
                        }
                        let uid = *uid;

                        match client.fetch_email(&db_folder.imap_path, uid) {
                            Ok(parsed) => {
                                let to_addrs: Vec<EmailAddress> = parsed
                                    .to_addresses
                                    .iter()
                                    .filter_map(|(addr, name)| {
                                        addr.as_ref().map(|a| EmailAddress {
                                            email: a.clone(),
                                            name: name.clone(),
                                        })
                                    })
                                    .collect();

                                match handle.block_on(emails::insert_email_with_labels(
                                    db_conn.clone(),
                                    credential_id,
                                    parsed.uid,
                                    db_folder.id,
                                    parsed.message_id.as_deref(),
                                    parsed.subject.as_deref(),
                                    &parsed.from_address.unwrap_or_default(),
                                    parsed.from_name.as_deref(),
                                    &to_addrs,
                                    &[],
                                    &[],
                                    parsed.reply_to.as_deref(),
                                    parsed.date_sent,
                                    parsed.date_received,
                                    parsed.body_text.as_deref(),
                                    parsed.body_html.as_deref(),
                                    parsed.is_read,
                                    parsed.is_flagged,
                                    parsed.is_draft,
                                    parsed.is_answered,
                                    parsed.has_attachments,
                                    parsed.attachment_count,
                                    parsed.size_bytes,
                                    &parsed.labels,
                                )) {
                                    Ok(email_id) => {
                                        tracing::debug!(uid, email_id, "stored email");
                                        highest_uid =
                                            Some(highest_uid.map_or(uid, |h: u32| h.max(uid)));
                                        lowest_uid =
                                            Some(lowest_uid.map_or(uid, |l: u32| l.min(uid)));
                                    }
                                    Err(e) => {
                                        tracing::error!(uid, error = %e, "failed to store email");
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(uid, error = %e, "failed to fetch email");
                            }
                        }
                    }

                    // Update folder sync position
                    match direction.as_ref() {
                        EmailSyncDirection::Recent => {
                            if let Some(uid) = highest_uid {
                                handle.block_on(folders::update_folder_sync_state(
                                    db_conn.clone(),
                                    db_folder.id,
                                    mailbox_meta.uidvalidity,
                                    uid,
                                ))?;
                                tracing::info!(
                                    folder = %db_folder.imap_path,
                                    last_synced_uid = uid,
                                    "updated recent sync position"
                                );
                            }
                        }
                        EmailSyncDirection::Backfill => {
                            if let Some(uid) = lowest_uid {
                                handle.block_on(folders::update_folder_backfill_state(
                                    db_conn.clone(),
                                    db_folder.id,
                                    uid,
                                ))?;
                                tracing::info!(
                                    folder = %db_folder.imap_path,
                                    oldest_synced_uid = uid,
                                    "updated backfill position"
                                );
                            }
                        }
                    }

                    Ok::<(), anyhow::Error>(())
                })
                .await
                .map_err(|e| anyhow::anyhow!("blocking task panicked: {}", e))?
            });
        }

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Err(e)) => tracing::warn!(error = %e, "folder sync task error"),
                Err(e) => tracing::warn!(error = %e, "folder sync task join error"),
                _ => {}
            }
        }

        tracing::info!(credential_id, direction = ?direction, "IMAP sync completed");
        Ok(())
    }
}
