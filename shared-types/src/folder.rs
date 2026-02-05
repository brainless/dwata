use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailFolder {
    pub id: i64,
    pub credential_id: i64,
    pub name: String,
    pub display_name: Option<String>,
    pub imap_path: String,
    pub folder_type: Option<String>,
    pub parent_folder_id: Option<i64>,
    pub uidvalidity: Option<u32>,
    pub last_synced_uid: Option<u32>,
    pub total_messages: i32,
    pub unread_messages: i32,
    pub is_subscribed: bool,
    pub is_selectable: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_synced_at: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ListFoldersRequest {
    pub credential_id: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ListFoldersResponse {
    pub folders: Vec<EmailFolder>,
}
