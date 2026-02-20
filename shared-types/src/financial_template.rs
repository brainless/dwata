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
