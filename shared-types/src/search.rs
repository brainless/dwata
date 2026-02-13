use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Document, DocumentKind};

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SearchField {
    Any,
    Title,
    FromAddress,
    BodyText,
    AttachmentText,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchTerm {
    pub field: SearchField,
    pub value: String,
    pub is_phrase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchDocumentsRequest {
    pub terms: Vec<SearchTerm>,
    pub kind: Option<DocumentKind>,
    pub source_id: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchHit {
    pub document_id: i64,
    pub score: f32,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchDocumentsResponse {
    pub hits: Vec<SearchHit>,
    pub documents: Vec<Document>,
    pub total_hits: usize,
}
