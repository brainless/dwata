use serde::{Deserialize, Serialize};

/// What type of content to search for
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SearchTarget {
    Email,
    File,
    All,
}

/// Which fields to search within
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchField {
    Any,
    Subject,     // Email subject or File title
    FromAddress, // Email sender
    ToAddresses, // Email recipients (to/cc)
    BodyText,    // Email body or File content
    Filename,    // File name (for files)
}

/// A single search term
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTerm {
    pub field: SearchField,
    pub value: String,
    pub is_phrase: bool,
}

/// Search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub target: SearchTarget,
    pub terms: Vec<SearchTerm>,
    pub credential_id: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Identifies what type of item was found and its ID
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HitId {
    Email(i64),
    File(i64),
}

/// A single search result hit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub hit_id: HitId,
    pub score: f32,
    pub snippet: Option<String>,
    // Preview fields to avoid extra fetch
    pub subject: Option<String>,
    pub filename: Option<String>,
    pub from_address: Option<String>,
    pub date_received: Option<i64>,
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total_hits: usize,
}
