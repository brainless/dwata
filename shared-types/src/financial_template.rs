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
    pub max_senders: Option<usize>,
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
