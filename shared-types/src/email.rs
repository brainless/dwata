use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents a stored email
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Email {
    pub id: i64,
    pub download_item_id: Option<i64>,

    // IMAP Metadata
    pub uid: u32,
    pub folder: String,
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
    pub is_answered: bool,

    // Metadata
    pub has_attachments: bool,
    pub attachment_count: i32,
    pub size_bytes: Option<i32>,
    pub thread_id: Option<String>,
    pub labels: Vec<String>,

    // Timestamps
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailAttachment {
    pub id: i64,
    pub email_id: i64,
    pub filename: String,
    pub content_type: Option<String>,
    pub size_bytes: Option<i32>,
    pub content_id: Option<String>,
    pub file_path: String,
    pub checksum: Option<String>,
    pub is_inline: bool,
    pub extraction_status: AttachmentExtractionStatus,
    pub extracted_text: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum AttachmentExtractionStatus {
    Pending,
    Completed,
    Failed,
    Skipped,
}

/// Request to list emails
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ListEmailsRequest {
    pub folder: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search_query: Option<String>,
}

/// Response for email list
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ListEmailsResponse {
    pub emails: Vec<Email>,
    pub total_count: i64,
    pub has_more: bool,
}
