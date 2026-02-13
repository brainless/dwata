use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentSourceType {
    ImapAccount,
    LocalFolder,
    CloudDrive,
    CloudMailbox,
    ManualImport,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum SourceAccessState {
    Accessible,
    Offline,
    Unreachable,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum SourcePermissionState {
    Granted,
    Expired,
    Revoked,
    InsufficientScope,
    Forbidden,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentKind {
    Email,
    Attachment,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DocumentSource {
    pub id: i64,
    pub source_type: DocumentSourceType,
    pub display_name: String,
    pub credential_id: Option<i64>,
    pub root_reference: Option<String>,
    pub access_state: SourceAccessState,
    pub permission_state: SourcePermissionState,
    pub access_checked_at: Option<i64>,
    pub permission_checked_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Document {
    pub id: i64,
    pub source_id: i64,
    pub kind: DocumentKind,
    pub parent_document_id: Option<i64>,
    pub email_id: Option<i64>,
    pub attachment_id: Option<i64>,
    pub title: Option<String>,
    pub canonical_name: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub checksum_sha256: Option<String>,
    pub storage_path: Option<String>,
    pub external_uri: Option<String>,
    pub date_created: Option<i64>,
    pub date_modified: Option<i64>,
    pub date_received: Option<i64>,
    pub indexed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
