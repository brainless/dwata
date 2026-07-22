use serde::{Deserialize, Serialize};

/// Represents a stored email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: i64,
    pub credential_id: i64,

    // IMAP Metadata
    pub uid: u32,
    pub folder_id: i64,
    pub message_id: Option<String>,

    // Headers
    pub subject: Option<String>,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to_addresses: Vec<EmailAddress>,
    pub cc_addresses: Vec<EmailAddress>,
    pub bcc_addresses: Vec<EmailAddress>,
    pub reply_to: Option<String>,

    // Dates
    pub date_sent: Option<i64>,
    pub date_received: i64,

    // Content
    pub body_text: Option<String>,
    pub body_html: Option<String>,

    // Flags
    pub is_read: bool,
    pub is_flagged: bool,
    pub is_draft: bool,

    // Metadata
    pub has_attachments: bool,
    pub attachment_count: i32,
    pub size_bytes: Option<i32>,
    pub thread_id: Option<String>,

    // Timestamps
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

/// Request to list emails
#[derive(Debug, Deserialize)]
pub struct ListEmailsRequest {
    pub credential_id: Option<i64>,
    pub folder_id: Option<i64>,
    pub label_id: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search_query: Option<String>,
}

/// Response for email list
#[derive(Debug, Serialize)]
pub struct ListEmailsResponse {
    pub emails: Vec<Email>,
    pub total_count: i64,
    pub has_more: bool,
}

/// Request to fetch emails by IDs
#[derive(Debug, Deserialize)]
pub struct EmailsByIdsRequest {
    pub email_ids: Vec<i64>,
}

/// Response for email batch lookup
#[derive(Debug, Serialize)]
pub struct EmailsByIdsResponse {
    pub emails: Vec<Email>,
}
