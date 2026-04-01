use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tracks extraction progress for a single account
#[derive(Debug, Clone)]
pub struct AccountProgress {
    pub credential_id: i64,
    pub identifier: String,
    pub status: ExtractionStatus,
    pub total_emails: i64,
    pub emails_processed: usize,
    pub emails_failed: usize,
    pub current_email_id: Option<i64>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtractionStatus {
    Idle,
    Running,
    Completed,
    Failed,
}

impl ExtractionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtractionStatus::Idle => "idle",
            ExtractionStatus::Running => "running",
            ExtractionStatus::Completed => "completed",
            ExtractionStatus::Failed => "failed",
        }
    }
}

/// Long polling state for tracking changes
#[derive(Debug, Clone)]
struct PollingState {
    last_update_timestamp: i64,
    last_progress_hash: u64,
}

/// Thread-safe extraction state manager with long polling support
#[derive(Debug, Clone)]
pub struct KgExtractionState {
    accounts: Arc<RwLock<HashMap<i64, AccountProgress>>>,
    global_active: Arc<RwLock<bool>>,
    polling_state: Arc<RwLock<PollingState>>,
}

impl KgExtractionState {
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            global_active: Arc::new(RwLock::new(false)),
            polling_state: Arc::new(RwLock::new(PollingState {
                last_update_timestamp: 0,
                last_progress_hash: 0,
            })),
        }
    }

    fn get_current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    fn compute_progress_hash(accounts: &HashMap<i64, AccountProgress>) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash the number of accounts first
        accounts.len().hash(&mut hasher);

        // Hash each account's progress (sorted by key for consistency)
        let mut sorted_keys: Vec<&i64> = accounts.keys().collect();
        sorted_keys.sort();

        for key in sorted_keys {
            if let Some(progress) = accounts.get(key) {
                key.hash(&mut hasher);
                progress.hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    fn update_polling_state(&self) {
        if let Ok(accounts) = self.accounts.read() {
            let hash = Self::compute_progress_hash(&accounts);
            let timestamp = Self::get_current_timestamp();

            if let Ok(mut state) = self.polling_state.write() {
                state.last_progress_hash = hash;
                state.last_update_timestamp = timestamp;
            }
        }
    }

    /// Wait for progress updates (long polling)
    /// Returns true if there was an update, false if timeout
    pub async fn wait_for_updates(&self, timeout_secs: u64) -> bool {
        let start_time = Self::get_current_timestamp();
        let initial_hash = if let Ok(state) = self.polling_state.read() {
            state.last_progress_hash
        } else {
            0
        };

        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
        let timeout_duration = tokio::time::Duration::from_secs(timeout_secs);
        let start_instant = tokio::time::Instant::now();

        loop {
            interval.tick().await;

            // Check if timeout
            if start_instant.elapsed() >= timeout_duration {
                return false;
            }

            // Check if there's been an update
            if let Ok(state) = self.polling_state.read() {
                if state.last_progress_hash != initial_hash {
                    return true;
                }
            }

            // Also check if extraction became inactive while we were waiting
            if !self.is_active() && initial_hash == 0 {
                return true;
            }
        }
    }

    /// Initialize account state before processing
    pub fn start_account(&self, credential_id: i64, identifier: String, total_emails: i64) {
        let now = chrono::Utc::now().timestamp();
        let progress = AccountProgress {
            credential_id,
            identifier,
            status: ExtractionStatus::Running,
            total_emails,
            emails_processed: 0,
            emails_failed: 0,
            current_email_id: None,
            started_at: Some(now),
            completed_at: None,
            error_message: None,
        };

        if let Ok(mut accounts) = self.accounts.write() {
            accounts.insert(credential_id, progress);
        }

        if let Ok(mut active) = self.global_active.write() {
            *active = true;
        }

        self.update_polling_state();
    }

    /// Update progress for an account
    pub fn update_account_progress(&self, credential_id: i64, email_id: i64, success: bool) {
        if let Ok(mut accounts) = self.accounts.write() {
            if let Some(progress) = accounts.get_mut(&credential_id) {
                progress.current_email_id = Some(email_id);
                if success {
                    progress.emails_processed += 1;
                } else {
                    progress.emails_failed += 1;
                }
            }
        }

        self.update_polling_state();
    }

    /// Mark account as completed
    pub fn complete_account(&self, credential_id: i64) {
        let now = chrono::Utc::now().timestamp();

        if let Ok(mut accounts) = self.accounts.write() {
            if let Some(progress) = accounts.get_mut(&credential_id) {
                progress.status = ExtractionStatus::Completed;
                progress.completed_at = Some(now);
                progress.current_email_id = None;
            }
        }

        // Check if all accounts are done
        if let Ok(accounts) = self.accounts.read() {
            let all_done = accounts.values().all(|p| {
                matches!(
                    p.status,
                    ExtractionStatus::Completed | ExtractionStatus::Failed
                )
            });

            if all_done {
                if let Ok(mut active) = self.global_active.write() {
                    *active = false;
                }
            }
        }

        self.update_polling_state();
    }

    /// Mark account as failed
    pub fn fail_account(&self, credential_id: i64, error: String) {
        let now = chrono::Utc::now().timestamp();

        if let Ok(mut accounts) = self.accounts.write() {
            if let Some(progress) = accounts.get_mut(&credential_id) {
                progress.status = ExtractionStatus::Failed;
                progress.error_message = Some(error);
                progress.completed_at = Some(now);
                progress.current_email_id = None;
            }
        }

        if let Ok(mut active) = self.global_active.write() {
            *active = false;
        }

        self.update_polling_state();
    }

    /// Get progress for all accounts
    pub fn get_all_progress(&self) -> Vec<AccountProgress> {
        if let Ok(accounts) = self.accounts.read() {
            accounts.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get progress for a specific account
    pub fn get_account_progress(&self, credential_id: i64) -> Option<AccountProgress> {
        if let Ok(accounts) = self.accounts.read() {
            accounts.get(&credential_id).cloned()
        } else {
            None
        }
    }

    /// Check if any extraction is currently running
    pub fn is_active(&self) -> bool {
        if let Ok(active) = self.global_active.read() {
            *active
        } else {
            false
        }
    }

    /// Clear completed/failed entries older than a certain time (optional cleanup)
    pub fn clear_old_entries(&self, _older_than_seconds: i64) {
        // Can be implemented if needed to prevent memory growth
        // For now, we keep all history
    }

    /// Notify that an update has occurred (used by ExtractionStateProvider)
    /// This updates the polling state to trigger long-polling responses
    pub fn notify_update(&self) {
        self.update_polling_state();
    }
}

impl Default for KgExtractionState {
    fn default() -> Self {
        Self::new()
    }
}

impl Hash for AccountProgress {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.credential_id.hash(state);
        self.status.as_str().hash(state);
        self.emails_processed.hash(state);
        self.emails_failed.hash(state);
        self.current_email_id.hash(state);
    }
}
