use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Person with email count statistics derived from the emails table.
#[derive(Debug, Serialize)]
pub struct PersonWithCounts {
    #[serde(flatten)]
    pub person: Person,
    /// Emails where this person is the sender (`from_address` match).
    pub received_count: i64,
    /// Emails where this person appears in `to_addresses` or `cc_addresses`.
    pub in_to_count: i64,
}

#[derive(Debug, Serialize)]
pub struct PersonsWithCountsResponse {
    pub persons: Vec<PersonWithCounts>,
}
