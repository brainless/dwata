use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::entity_search::EntitySearchResult;
use crate::entity_types::ExtractedEntitiesParams;
use crate::kg_email_extractor::types::LabelDocumentParams;
use crate::kg_pass_context::KgPassType;

/// Events that occur during extraction, tracked step by step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "step_type", rename_all = "snake_case")]
pub enum ExtractionStep {
    /// Document labeling completed
    DocumentLabeled {
        timestamp: i64,
        label: LabelDocumentParams,
    },

    /// A pass is starting
    PassStarted {
        timestamp: i64,
        pass_type: KgPassType,
        pass_name: String,
    },

    /// Search performed for pre-population
    SearchPerformed {
        timestamp: i64,
        pass_type: KgPassType,
        keywords: String,
        entity_types: Vec<String>,
        results: Vec<EntitySearchResult>,
        result_count: usize,
    },

    /// Search performed by sender email
    SenderSearchPerformed {
        timestamp: i64,
        pass_type: KgPassType,
        sender_email: String,
        results: Vec<EntitySearchResult>,
        result_count: usize,
    },

    /// Entities extracted from a pass
    EntitiesExtracted {
        timestamp: i64,
        pass_type: KgPassType,
        entities: ExtractedEntitiesParams,
        entity_counts: HashMap<String, usize>,
        total_entities: usize,
    },

    /// Pass completed successfully
    PassCompleted {
        timestamp: i64,
        pass_type: KgPassType,
        entities_persisted: bool,
    },

    /// Pass failed with error
    PassFailed {
        timestamp: i64,
        pass_type: KgPassType,
        error_message: String,
    },

    /// Tool call made during extraction
    ToolCallMade {
        timestamp: i64,
        pass_type: KgPassType,
        tool_name: String,
        iteration: usize,
    },

    /// Retry occurred due to parse failure or other issue
    RetryOccurred {
        timestamp: i64,
        pass_type: KgPassType,
        reason: RetryReason,
        attempt: usize,
        max_attempts: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryReason {
    ParseFailed,
    ConfirmBeforeSubmit,
    EmptyConfirm,
    NoToolCalls,
}

/// Summary statistics for an extraction run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSummary {
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub email_id: Option<i64>,
    pub sender_email: Option<String>,
    pub status: ExtractionStatus,
    pub total_passes: usize,
    pub completed_passes: usize,
    pub failed_passes: usize,
    pub total_entities_extracted: usize,
    pub total_search_results: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionStatus {
    Running,
    Completed,
    Failed,
}

impl ExtractionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtractionStatus::Running => "running",
            ExtractionStatus::Completed => "completed",
            ExtractionStatus::Failed => "failed",
        }
    }
}

/// Complete state for a single email extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionStepState {
    pub session_id: i64,
    pub summary: ExtractionSummary,
    pub steps: Vec<ExtractionStep>,
    pub pass_states: HashMap<String, PassStepState>,
}

/// Per-pass state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassStepState {
    pub pass_type: KgPassType,
    pub pass_name: String,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub status: PassStatus,
    pub search_keywords: Option<String>,
    pub search_results: Vec<EntitySearchResult>,
    pub entities_extracted: Option<ExtractedEntitiesParams>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PassStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl PassStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PassStatus::Pending => "pending",
            PassStatus::Running => "running",
            PassStatus::Completed => "completed",
            PassStatus::Failed => "failed",
        }
    }
}

impl ExtractionStepState {
    pub fn new(session_id: i64, email_id: Option<i64>, sender_email: Option<String>) -> Self {
        Self {
            session_id,
            summary: ExtractionSummary {
                started_at: current_timestamp(),
                completed_at: None,
                email_id,
                sender_email,
                status: ExtractionStatus::Running,
                total_passes: 0,
                completed_passes: 0,
                failed_passes: 0,
                total_entities_extracted: 0,
                total_search_results: 0,
                error_message: None,
            },
            steps: Vec::new(),
            pass_states: HashMap::new(),
        }
    }

    pub fn record_step(&mut self, step: ExtractionStep) {
        // Update summary based on step type
        match &step {
            ExtractionStep::DocumentLabeled { .. } => {
                self.summary.total_passes += 1;
            }
            ExtractionStep::PassStarted {
                pass_type,
                pass_name,
                ..
            } => {
                let pass_key = pass_type.name().to_string();
                self.pass_states.insert(
                    pass_key.clone(),
                    PassStepState {
                        pass_type: *pass_type,
                        pass_name: pass_name.clone(),
                        started_at: Some(current_timestamp()),
                        completed_at: None,
                        status: PassStatus::Running,
                        search_keywords: None,
                        search_results: Vec::new(),
                        entities_extracted: None,
                        error_message: None,
                    },
                );
            }
            ExtractionStep::SearchPerformed {
                pass_type, results, ..
            } => {
                if let Some(pass_state) = self.pass_states.get_mut(pass_type.name()) {
                    pass_state.search_results.extend(results.clone());
                    self.summary.total_search_results += results.len();
                }
            }
            ExtractionStep::SenderSearchPerformed {
                pass_type, results, ..
            } => {
                if let Some(pass_state) = self.pass_states.get_mut(pass_type.name()) {
                    pass_state.search_results.extend(results.clone());
                    self.summary.total_search_results += results.len();
                }
            }
            ExtractionStep::EntitiesExtracted {
                pass_type,
                total_entities,
                ..
            } => {
                if let Some(pass_state) = self.pass_states.get_mut(pass_type.name()) {
                    // Store the entities in the pass state
                    // We need to access the entities field from the step
                    if let ExtractionStep::EntitiesExtracted { entities, .. } = &step {
                        pass_state.entities_extracted = Some(entities.clone());
                    }
                }
                self.summary.total_entities_extracted += total_entities;
            }
            ExtractionStep::PassCompleted { pass_type, .. } => {
                if let Some(pass_state) = self.pass_states.get_mut(pass_type.name()) {
                    pass_state.status = PassStatus::Completed;
                    pass_state.completed_at = Some(current_timestamp());
                }
                self.summary.completed_passes += 1;
            }
            ExtractionStep::PassFailed {
                pass_type,
                error_message,
                ..
            } => {
                if let Some(pass_state) = self.pass_states.get_mut(pass_type.name()) {
                    pass_state.status = PassStatus::Failed;
                    pass_state.error_message = Some(error_message.clone());
                    pass_state.completed_at = Some(current_timestamp());
                }
                self.summary.failed_passes += 1;
            }
            _ => {}
        }

        self.steps.push(step);
    }

    pub fn complete(&mut self) {
        self.summary.completed_at = Some(current_timestamp());
        self.summary.status =
            if self.summary.failed_passes > 0 && self.summary.completed_passes == 0 {
                ExtractionStatus::Failed
            } else {
                ExtractionStatus::Completed
            };
    }

    pub fn fail(&mut self, error_message: String) {
        self.summary.completed_at = Some(current_timestamp());
        self.summary.status = ExtractionStatus::Failed;
        self.summary.error_message = Some(error_message);
    }

    pub fn get_pass_state(&self, pass_type: &KgPassType) -> Option<&PassStepState> {
        self.pass_states.get(pass_type.name())
    }

    pub fn get_entity_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();

        for step in &self.steps {
            if let ExtractionStep::EntitiesExtracted { entity_counts, .. } = step {
                for (entity_type, count) in entity_counts {
                    *counts.entry(entity_type.clone()).or_insert(0) += *count;
                }
            }
        }

        counts
    }
}

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

/// Helper function to count entities by type
pub fn count_entities_by_type(
    entities: &ExtractedEntitiesParams,
) -> (HashMap<String, usize>, usize) {
    let mut counts = HashMap::new();
    let mut total = 0;

    if let Some(locations) = &entities.locations {
        let count = locations.len();
        if count > 0 {
            counts.insert("locations".to_string(), count);
            total += count;
        }
    }
    if let Some(organisations) = &entities.organisations {
        let count = organisations.len();
        if count > 0 {
            counts.insert("organisations".to_string(), count);
            total += count;
        }
    }
    if let Some(persons) = &entities.persons {
        let count = persons.len();
        if count > 0 {
            counts.insert("persons".to_string(), count);
            total += count;
        }
    }
    if let Some(bills) = &entities.bills {
        let count = bills.len();
        if count > 0 {
            counts.insert("bills".to_string(), count);
            total += count;
        }
    }
    if let Some(transactions) = &entities.transactions {
        let count = transactions.len();
        if count > 0 {
            counts.insert("transactions".to_string(), count);
            total += count;
        }
    }
    if let Some(subscriptions) = &entities.subscriptions {
        let count = subscriptions.len();
        if count > 0 {
            counts.insert("subscriptions".to_string(), count);
            total += count;
        }
    }
    if let Some(orders) = &entities.orders {
        let count = orders.len();
        if count > 0 {
            counts.insert("orders".to_string(), count);
            total += count;
        }
    }
    if let Some(events) = &entities.events {
        let count = events.len();
        if count > 0 {
            counts.insert("events".to_string(), count);
            total += count;
        }
    }

    (counts, total)
}

impl Hash for ExtractionStepState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.session_id.hash(state);
        self.summary.started_at.hash(state);
        self.summary.status.as_str().hash(state);
        self.summary.total_entities_extracted.hash(state);
        self.summary.total_passes.hash(state);
        self.summary.completed_passes.hash(state);
    }
}
