use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

// Re-export all extraction types from shared_types
pub use shared_types::{
    count_entities_by_type, ExtractionStatus, ExtractionStep, ExtractionStepState,
    ExtractionSummary, PassStatus, PassStepState, RetryReason,
};

/// Trait for providing extraction state updates
#[async_trait]
pub trait ExtractionStateProvider: Send + Sync {
    /// Record a step event
    async fn record_step(&self, session_id: i64, step: ExtractionStep);

    /// Get the current extraction state for a session
    async fn get_state(&self, session_id: i64) -> Option<ExtractionStepState>;

    /// Initialize extraction state for a new session
    async fn initialize_state(
        &self,
        session_id: i64,
        email_id: Option<i64>,
        sender_email: Option<String>,
    );

    /// Mark extraction as complete
    async fn complete_extraction(&self, session_id: i64);

    /// Mark extraction as failed
    async fn fail_extraction(&self, session_id: i64, error_message: String);
}

/// In-memory implementation of extraction state provider
#[derive(Debug, Clone)]
pub struct InMemoryExtractionState {
    states: Arc<RwLock<HashMap<i64, ExtractionStepState>>>,
}

impl InMemoryExtractionState {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_all_states(&self) -> Vec<ExtractionStepState> {
        if let Ok(states) = self.states.read() {
            states.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn clear_old_states(&self, older_than_seconds: i64) {
        let cutoff = current_timestamp() - older_than_seconds;
        if let Ok(mut states) = self.states.write() {
            states.retain(|_, state| {
                let should_keep = match state.summary.completed_at {
                    Some(completed_at) => completed_at > cutoff,
                    None => true, // Keep running extractions
                };
                should_keep
            });
        }
    }
}

impl Default for InMemoryExtractionState {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExtractionStateProvider for InMemoryExtractionState {
    async fn record_step(&self, session_id: i64, step: ExtractionStep) {
        if let Ok(mut states) = self.states.write() {
            if let Some(state) = states.get_mut(&session_id) {
                state.record_step(step);
            }
        }
    }

    async fn get_state(&self, session_id: i64) -> Option<ExtractionStepState> {
        if let Ok(states) = self.states.read() {
            states.get(&session_id).cloned()
        } else {
            None
        }
    }

    async fn initialize_state(
        &self,
        session_id: i64,
        email_id: Option<i64>,
        sender_email: Option<String>,
    ) {
        let state = ExtractionStepState::new(session_id, email_id, sender_email);
        if let Ok(mut states) = self.states.write() {
            states.insert(session_id, state);
        }
    }

    async fn complete_extraction(&self, session_id: i64) {
        if let Ok(mut states) = self.states.write() {
            if let Some(state) = states.get_mut(&session_id) {
                state.complete();
            }
        }
    }

    async fn fail_extraction(&self, session_id: i64, error_message: String) {
        if let Ok(mut states) = self.states.write() {
            if let Some(state) = states.get_mut(&session_id) {
                state.fail(error_message);
            }
        }
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
