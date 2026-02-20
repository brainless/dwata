use shared_types::{FinancialTemplateDetectionJobState, FinancialTemplateDetectionJobStatus};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TemplateDetectionJobState {
    inner: Arc<Mutex<FinancialTemplateDetectionJobState>>,
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
            })),
        }
    }

    pub fn snapshot(&self) -> FinancialTemplateDetectionJobState {
        self.inner.lock().expect("job state lock poisoned").clone()
    }

    pub fn with_mut<F>(&self, f: F) -> FinancialTemplateDetectionJobState
    where
        F: FnOnce(&mut FinancialTemplateDetectionJobState),
    {
        let mut guard = self.inner.lock().expect("job state lock poisoned");
        f(&mut guard);
        guard.clone()
    }
}
