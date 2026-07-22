use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::document_label::LabelDocumentParams;
use crate::entity_types::ExtractedEntitiesParams;
use crate::kg_extraction::EntitySearchResult;
use crate::kg_pass::KgPassType;

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

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
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
