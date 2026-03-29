use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Person {
    pub id: i64,
    pub email_id: Option<i64>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organisation_id: Option<i64>,
    /// LLM-generated summary for BM25 search during future extraction passes.
    /// Should capture relational context: e.g. "engineer at Acme Corp, john@acme.com"
    pub search_summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PersonsResponse {
    pub persons: Vec<Person>,
}
