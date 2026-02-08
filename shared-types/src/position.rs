use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Position {
    pub id: i64,
    pub extraction_job_id: Option<i64>,
    pub contact_id: i64,
    pub company_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub started_on: Option<String>,
    pub finished_on: Option<String>,
    pub started_date: Option<i64>,
    pub finished_date: Option<i64>,
    pub is_current: bool,
    pub confidence: Option<f32>,
    pub requires_review: bool,
    pub is_confirmed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreatePositionRequest {
    pub contact_id: i64,
    pub company_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub started_on: Option<String>,
    pub finished_on: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PositionsResponse {
    pub positions: Vec<Position>,
}
