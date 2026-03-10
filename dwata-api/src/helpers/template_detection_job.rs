use shared_types::{FinancialTemplateDetectionJobState, FinancialTemplateDetectionJobStatus};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tokio::time::{timeout, Duration};

#[derive(Clone)]
pub struct TemplateDetectionJobState {
    inner: Arc<Mutex<FinancialTemplateDetectionJobState>>,
    version: Arc<AtomicU64>,
    notify: Arc<Notify>,
}

impl TemplateDetectionJobState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(FinancialTemplateDetectionJobState {
                run_id: 0,
                status: FinancialTemplateDetectionJobStatus::Idle,
                started_at: None,
                finished_at: None,
                total_senders: 0,
                processed_senders: 0,
                current_sender: None,
                candidate_sender_count: 0,
                candidate_email_count: 0,
                new_templates_count: 0,
                error: None,
                debug: None,
            })),
            version: Arc::new(AtomicU64::new(1)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn snapshot(&self) -> FinancialTemplateDetectionJobState {
        self.inner.lock().expect("job state lock poisoned").clone()
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    pub fn snapshot_with_version(&self) -> (FinancialTemplateDetectionJobState, u64) {
        (self.snapshot(), self.version())
    }

    pub fn with_mut<F>(&self, f: F) -> FinancialTemplateDetectionJobState
    where
        F: FnOnce(&mut FinancialTemplateDetectionJobState),
    {
        let mut guard = self.inner.lock().expect("job state lock poisoned");
        f(&mut guard);
        let snapshot = guard.clone();
        self.version.fetch_add(1, Ordering::AcqRel);
        self.notify.notify_waiters();
        snapshot
    }

    pub async fn wait_for_change(
        &self,
        since_version: Option<u64>,
        timeout_ms: u64,
    ) -> (FinancialTemplateDetectionJobState, u64) {
        let effective_timeout_ms = timeout_ms.clamp(1_000, 30_000);
        if let Some(since) = since_version {
            if self.version() <= since {
                let _ = timeout(
                    Duration::from_millis(effective_timeout_ms),
                    self.notify.notified(),
                )
                .await;
            }
        }
        self.snapshot_with_version()
    }
}
