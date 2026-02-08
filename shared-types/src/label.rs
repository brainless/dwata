use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailLabel {
    pub id: i64,
    pub credential_id: i64,
    pub name: String,
    pub display_name: Option<String>,
    pub label_type: String,
    pub color: Option<String>,
    pub message_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ListLabelsRequest {
    pub credential_id: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ListLabelsResponse {
    pub labels: Vec<EmailLabel>,
}
