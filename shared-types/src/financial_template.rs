use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::transaction::DataSourceType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum FinancialTemplateType {
    Bill,
    Transaction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum FinancialTemplateStatus {
    Active,
    Superseded,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialExtractionTemplate {
    pub id: i64,
    pub data_source_type: DataSourceType,
    pub data_source_id: String,
    pub template_type: FinancialTemplateType,
    pub template_body: String,
    pub status: FinancialTemplateStatus,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialTemplateVariable {
    pub id: i64,
    pub template_id: i64,
    pub placeholder_name: String,
    pub target_field: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialTemplateApplicability {
    pub id: i64,
    pub template_id: i64,
    pub data_source_type: DataSourceType,
    pub data_source_id: String,
    pub match_score: Option<f64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DetectFinancialTemplatesRequest {
    pub credential_id: Option<i64>,
    pub max_candidate_emails: Option<usize>,
    pub max_templates_per_sender: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DetectedFinancialTemplateVariable {
    pub placeholder_name: String,
    pub target_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DetectedFinancialTemplate {
    pub template_id: i64,
    pub sender_email: String,
    pub template_type: FinancialTemplateType,
    pub template_body: String,
    pub translated_template_body: String,
    pub source_email_ids: Vec<i64>,
    pub variables: Vec<DetectedFinancialTemplateVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DetectFinancialTemplatesResponse {
    pub candidate_sender_count: usize,
    pub candidate_email_count: usize,
    pub templates: Vec<DetectedFinancialTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TemplateDetectionSenderRank {
    pub sender_email: String,
    pub rank: usize,
    pub total_candidate_emails: usize,
    pub recent_candidate_emails: usize,
    pub latest_email_ts: i64,
    pub max_existing_cluster_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TemplateDetectionGeneratedTemplateDebug {
    pub template_id: Option<i64>,
    pub template_type: Option<FinancialTemplateType>,
    pub template_body: String,
    pub translated_template_body: String,
    pub source_email_ids: Vec<i64>,
    pub variables: Vec<DetectedFinancialTemplateVariable>,
    pub has_bill: bool,
    pub discarded_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TemplateDetectionSenderDebug {
    pub sender_email: String,
    pub rank: usize,
    pub sender_candidate_count: usize,
    pub existing_template_count: usize,
    pub initially_matched_count: usize,
    pub fresh_unmatched_count: usize,
    pub pool_count: usize,
    pub generated_templates: Vec<TemplateDetectionGeneratedTemplateDebug>,
    pub error: Option<String>,
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TemplateDetectionDebugState {
    pub keyword_query: String,
    pub keyword_list: Vec<String>,
    pub max_candidate_emails: usize,
    pub matched_email_ids_count: usize,
    pub sender_ranking: Vec<TemplateDetectionSenderRank>,
    pub candidate_email_ids: Vec<i64>,
    pub sender_debug: Vec<TemplateDetectionSenderDebug>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum FinancialTemplateDetectionJobStatus {
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialTemplateDetectionJobState {
    pub run_id: i64,
    pub status: FinancialTemplateDetectionJobStatus,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub total_senders: usize,
    pub processed_senders: usize,
    pub current_sender: Option<String>,
    pub candidate_sender_count: usize,
    pub candidate_email_count: usize,
    pub new_templates_count: usize,
    pub error: Option<String>,
    pub debug: Option<TemplateDetectionDebugState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TemplateDetectionSenderLlmDraftPreview {
    pub seed_text: String,
    pub cluster_size: usize,
    pub selected_email_ids: Vec<i64>,
    pub full_template: String,
    pub sample_subject: String,
    pub sample_body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TemplateDetectionSenderLlmInputsResponse {
    pub sender_email: String,
    pub sender_candidate_count: usize,
    pub existing_template_count: usize,
    pub initially_matched_count: usize,
    pub fresh_unmatched_count: usize,
    pub pool_count: usize,
    pub drafts: Vec<TemplateDetectionSenderLlmDraftPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialTemplateFieldMapping {
    pub placeholder_name: String,
    pub target_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialTemplateWithVariables {
    pub template: FinancialExtractionTemplate,
    pub variables: Vec<FinancialTemplateFieldMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ListFinancialTemplatesResponse {
    pub templates: Vec<FinancialTemplateWithVariables>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DeleteFinancialTemplatesRequest {
    pub template_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DeleteFinancialTemplatesResponse {
    pub deleted_count: usize,
}
