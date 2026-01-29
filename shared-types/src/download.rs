use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents a long-running download job
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DownloadJob {
    pub id: i64,
    pub source_type: SourceType,
    pub credential_id: i64,
    pub status: DownloadJobStatus,
    pub progress: DownloadProgress,
    #[ts(skip)]
    pub source_state: serde_json::Value,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
    pub last_sync_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum SourceType {
    Imap,
    GoogleDrive,
    Dropbox,
    OneDrive,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadJobStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DownloadProgress {
    pub total_items: u64,
    pub downloaded_items: u64,
    pub failed_items: u64,
    pub skipped_items: u64,
    pub in_progress_items: u64,
    pub remaining_items: u64,
    pub percent_complete: f32,
    pub bytes_downloaded: u64,
    pub items_per_second: f32,
    pub estimated_completion_secs: Option<u64>,
}

/// IMAP-specific download state
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ImapDownloadState {
    pub folders: Vec<ImapFolderStatus>,
    pub sync_strategy: ImapSyncStrategy,
    #[ts(skip)]
    pub last_highest_uid: serde_json::Value,
    pub fetch_batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ImapFolderStatus {
    pub name: String,
    pub total_messages: u32,
    pub downloaded_messages: u32,
    pub failed_messages: u32,
    pub skipped_messages: u32,
    pub last_synced_uid: Option<u32>,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ImapSyncStrategy {
    FullSync,
    InboxOnly,
    SelectedFolders(Vec<String>),
    NewOnly,
    DateRange { from: String, to: String },
}

/// Cloud storage-specific state (for future)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CloudStorageDownloadState {
    pub root_path: String,
    pub directories: Vec<DirectoryStatus>,
    pub file_filter: Option<FileFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DirectoryStatus {
    pub path: String,
    pub total_files: u32,
    pub downloaded_files: u32,
    pub failed_files: u32,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FileFilter {
    pub extensions: Option<Vec<String>>,
    pub pattern: Option<String>,
    pub min_size_bytes: Option<u64>,
    pub max_size_bytes: Option<u64>,
}

/// Request to create a new download job
#[derive(Debug, Deserialize, TS)]
pub struct CreateDownloadJobRequest {
    pub credential_id: i64,
    pub source_type: SourceType,
    #[ts(skip)]
    pub source_config: serde_json::Value,
}

/// Response for download job list
#[derive(Debug, Serialize, TS)]
pub struct DownloadJobListResponse {
    pub jobs: Vec<DownloadJob>,
}

/// Individual download item
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DownloadItem {
    pub id: i64,
    pub job_id: i64,
    pub source_identifier: String,
    pub source_folder: Option<String>,
    pub item_type: String,
    pub status: DownloadItemStatus,
    pub size_bytes: Option<i64>,
    #[ts(skip)]
    pub metadata: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub downloaded_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadItemStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
    Skipped,
}
