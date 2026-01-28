# Task: IMAP Download Manager with Status Tracking

## Objective

Implement a comprehensive download management system for IMAP email ingestion that tracks progress, handles resumption after restarts, detects new emails dynamically, and provides a foundation for future cloud storage downloads (Google Drive, Dropbox, OneDrive, etc.).

## Background

### Current State
- **dwata**: Has IMAP credentials stored securely (OS keychain + DuckDB metadata)
- **nocodo**: Provides IMAP tools for mailbox operations (list, status, search, fetch headers, fetch email)
- **Gap**: No background download manager, no status tracking, no automatic sync

### Requirements
1. **Automatic Downloads**: Background job that downloads emails from IMAP servers
2. **Progress Tracking**: Real-time status updates (total emails, downloaded, failed, skipped)
3. **Resumable**: After restart, continue from last checkpoint
4. **Dynamic Updates**: Detect new emails at source, update totals immediately
5. **Multi-Folder Support**: Track progress per IMAP folder (INBOX, Sent, etc.)
6. **Generic Design**: Architecture extensible to cloud storage (Google Drive files, etc.)
7. **API Access**: REST endpoints for starting/pausing/monitoring downloads

### Integration with nocodo
The nocodo project provides:
- **IMAP Tool** (`nocodo-tools/src/imap/`): Read-only IMAP operations
  - `ListMailboxes`: Get available folders
  - `MailboxStatus`: Get message counts per folder
  - `Search`: Find emails by criteria (date range, unseen, etc.)
  - `FetchHeaders`: Get metadata (subject, from, to, date, flags, size)
  - `FetchEmail`: Download full email content (text, HTML, attachments)

- **IMAP Agent** (`nocodo-agents/src/imap_email/`): AI-powered email analysis
  - Not needed for bulk download (tool-only approach is sufficient)

**Strategy**: Use nocodo IMAP tool directly for downloads, not the agent.

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                         GUI (Frontend)                       │
│   - Download job list                                        │
│   - Progress bars per folder                                 │
│   - Start/Pause/Cancel controls                              │
└─────────────────────────────────────────────────────────────┘
                          ↓ HTTP
┌─────────────────────────────────────────────────────────────┐
│                    API Handlers (dwata-api)                  │
│   POST   /api/downloads          - Create job               │
│   GET    /api/downloads          - List jobs                │
│   GET    /api/downloads/:id      - Get status               │
│   POST   /api/downloads/:id/start   - Start/resume          │
│   POST   /api/downloads/:id/pause   - Pause                 │
│   DELETE /api/downloads/:id      - Cancel/delete            │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│              DownloadManager (Background Jobs)               │
│   - Spawns Tokio tasks per job                              │
│   - Batch processing (100 emails at a time)                 │
│   - Periodic sync (check for new emails every 5 min)        │
│   - Checkpoint after each batch                              │
│   - Error handling & retry logic                             │
└─────────────────────────────────────────────────────────────┘
         ↓                                ↓
┌──────────────────────┐      ┌────────────────────────────┐
│  nocodo IMAP Tool    │      │    DuckDB Database         │
│  - List folders      │      │  - download_jobs table     │
│  - Check status      │      │  - download_items table    │
│  - Search emails     │      │  - Checkpoints for resume  │
│  - Fetch headers     │      │  - Progress tracking       │
│  - Fetch full email  │      └────────────────────────────┘
└──────────────────────┘
         ↓
┌──────────────────────┐
│  IMAP Server         │
│  (Gmail, Outlook,    │
│   Exchange, etc.)    │
└──────────────────────┘
```

### Data Flow

**Job Creation:**
1. User creates IMAP credential in GUI
2. User clicks "Download Emails" → API creates `DownloadJob`
3. Job starts in `pending` status
4. Background manager picks up job, starts download

**Download Execution:**
1. Get credentials from keychain
2. List IMAP folders via nocodo tool
3. For each folder:
   - Check mailbox status (total messages)
   - Search for UIDs to download
   - Fetch headers in batches (100 at a time)
   - Download full emails
   - Store in database
   - Update progress after each batch
4. Mark job as `completed`

**Periodic Sync:**
1. Every 5 minutes, check all active/completed jobs
2. For each IMAP account, check mailbox status
3. If new emails detected, update `total_items` in DB
4. If job was completed, optionally resume to fetch new emails

## Database Schema

### download_jobs Table

```sql
CREATE TABLE IF NOT EXISTS download_jobs (
    id VARCHAR PRIMARY KEY,                    -- job_XXXXXXXXXXXX
    source_type VARCHAR NOT NULL,              -- imap, google_drive, dropbox, etc.
    credential_id VARCHAR NOT NULL,            -- FK to credentials_metadata
    status VARCHAR NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'paused', 'completed', 'failed', 'cancelled')),

    -- Progress tracking
    total_items BIGINT NOT NULL DEFAULT 0,
    downloaded_items BIGINT NOT NULL DEFAULT 0,
    failed_items BIGINT NOT NULL DEFAULT 0,
    skipped_items BIGINT NOT NULL DEFAULT 0,
    in_progress_items BIGINT NOT NULL DEFAULT 0,
    bytes_downloaded BIGINT NOT NULL DEFAULT 0,

    -- Source-specific state (JSON)
    source_state VARCHAR NOT NULL,             -- ImapDownloadState, CloudStorageDownloadState, etc.

    -- Error handling
    error_message VARCHAR,
    retry_count INTEGER DEFAULT 0,

    -- Timestamps
    created_at BIGINT NOT NULL,
    started_at BIGINT,
    updated_at BIGINT NOT NULL,
    completed_at BIGINT,
    last_sync_at BIGINT,                       -- Last check for new items

    FOREIGN KEY (credential_id) REFERENCES credentials_metadata (id)
);

CREATE INDEX IF NOT EXISTS idx_download_jobs_status
    ON download_jobs(status, updated_at);

CREATE INDEX IF NOT EXISTS idx_download_jobs_credential
    ON download_jobs(credential_id);
```

### download_items Table

```sql
CREATE TABLE IF NOT EXISTS download_items (
    id VARCHAR PRIMARY KEY,                    -- item_XXXXXXXXXXXX
    job_id VARCHAR NOT NULL,
    source_identifier VARCHAR NOT NULL,        -- IMAP UID, file path, etc.
    source_folder VARCHAR,                     -- IMAP folder, directory path
    item_type VARCHAR NOT NULL,                -- email, file, folder
    status VARCHAR NOT NULL
        CHECK (status IN ('pending', 'downloading', 'completed', 'failed', 'skipped')),

    -- Metadata
    size_bytes BIGINT,
    mime_type VARCHAR,
    metadata VARCHAR,                          -- JSON with source-specific data

    -- Error handling
    error_message VARCHAR,
    retry_count INTEGER DEFAULT 0,
    last_attempt_at BIGINT,

    -- Storage location
    local_path VARCHAR,                        -- Where downloaded content is stored

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    downloaded_at BIGINT,

    FOREIGN KEY (job_id) REFERENCES download_jobs (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_download_items_job_status
    ON download_items(job_id, status);

CREATE INDEX IF NOT EXISTS idx_download_items_source_identifier
    ON download_items(job_id, source_identifier);
```

## Type Definitions

### Shared Types (shared-types/src/download.rs)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Represents a long-running download job
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DownloadJob {
    pub id: String,                          // job_XXXXXXXXXXXX
    pub source_type: SourceType,             // imap, google_drive, dropbox, etc.
    pub credential_id: String,               // Reference to credentials_metadata
    pub status: DownloadJobStatus,           // pending, running, paused, completed, failed
    pub progress: DownloadProgress,          // Counts and percentages
    pub source_state: serde_json::Value,     // Source-specific metadata (folders, paths)
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
    pub last_sync_at: Option<i64>,          // Last time we checked source for changes
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum SourceType {
    Imap,
    GoogleDrive,
    Dropbox,
    OneDrive,
    // Future: S3, FTP, WebDAV, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadJobStatus {
    Pending,        // Queued, not started
    Running,        // Actively downloading
    Paused,         // User paused
    Completed,      // Finished successfully
    Failed,         // Error occurred
    Cancelled,      // User cancelled
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DownloadProgress {
    pub total_items: u64,              // Total available at source
    pub downloaded_items: u64,         // Successfully downloaded
    pub failed_items: u64,             // Failed to download
    pub skipped_items: u64,            // Already existed, skipped
    pub in_progress_items: u64,        // Currently downloading

    // Derived fields
    pub remaining_items: u64,          // total - downloaded - failed - skipped
    pub percent_complete: f32,         // (downloaded + skipped) / total * 100

    // Performance metrics
    pub bytes_downloaded: u64,
    pub items_per_second: f32,         // Recent download rate
    pub estimated_completion_secs: Option<u64>,
}

/// IMAP-specific download state
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImapDownloadState {
    pub folders: Vec<ImapFolderStatus>,
    pub sync_strategy: ImapSyncStrategy,
    pub last_highest_uid: HashMap<String, u32>,  // folder -> highest UID seen
    pub fetch_batch_size: usize,                  // Default: 100
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImapFolderStatus {
    pub name: String,                   // "INBOX", "Sent", etc.
    pub total_messages: u32,            // From IMAP STATUS
    pub downloaded_messages: u32,
    pub failed_messages: u32,
    pub skipped_messages: u32,
    pub last_synced_uid: Option<u32>,  // Resume point
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ImapSyncStrategy {
    FullSync,           // Download all emails from all folders
    InboxOnly,          // Only INBOX
    SelectedFolders(Vec<String>),  // Specific folders
    NewOnly,            // Only unseen emails
    DateRange { from: String, to: String },  // Date-filtered
}

/// Cloud storage-specific state (for future)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CloudStorageDownloadState {
    pub root_path: String,
    pub directories: Vec<DirectoryStatus>,
    pub file_filter: Option<FileFilter>,  // *.pdf, images/*, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DirectoryStatus {
    pub path: String,
    pub total_files: u32,
    pub downloaded_files: u32,
    pub failed_files: u32,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileFilter {
    pub extensions: Option<Vec<String>>,  // ["pdf", "docx"]
    pub pattern: Option<String>,          // Regex pattern
    pub min_size_bytes: Option<u64>,
    pub max_size_bytes: Option<u64>,
}

/// Request to create a new download job
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateDownloadJobRequest {
    pub credential_id: String,
    pub source_type: SourceType,
    pub source_config: serde_json::Value,  // ImapSyncStrategy or CloudStorageConfig
}

/// Response for download job list
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct DownloadJobListResponse {
    pub jobs: Vec<DownloadJob>,
}

/// Individual download item
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DownloadItem {
    pub id: String,
    pub job_id: String,
    pub source_identifier: String,        // IMAP UID, file path, etc.
    pub source_folder: Option<String>,
    pub item_type: String,                // email, file, folder
    pub status: DownloadItemStatus,
    pub size_bytes: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub downloaded_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadItemStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
    Skipped,
}
```

## Implementation Plan

### Phase 1: Database Schema & Types

#### 1.1 Add Migration

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/database/migrations.rs`

Add to `run_migrations()`:
```rust
// Create download_jobs table
conn.execute(
    "CREATE TABLE IF NOT EXISTS download_jobs (
        id VARCHAR PRIMARY KEY,
        source_type VARCHAR NOT NULL,
        credential_id VARCHAR NOT NULL,
        status VARCHAR NOT NULL DEFAULT 'pending',
        total_items BIGINT NOT NULL DEFAULT 0,
        downloaded_items BIGINT NOT NULL DEFAULT 0,
        failed_items BIGINT NOT NULL DEFAULT 0,
        skipped_items BIGINT NOT NULL DEFAULT 0,
        in_progress_items BIGINT NOT NULL DEFAULT 0,
        bytes_downloaded BIGINT NOT NULL DEFAULT 0,
        source_state VARCHAR NOT NULL,
        error_message VARCHAR,
        retry_count INTEGER DEFAULT 0,
        created_at BIGINT NOT NULL,
        started_at BIGINT,
        updated_at BIGINT NOT NULL,
        completed_at BIGINT,
        last_sync_at BIGINT,
        FOREIGN KEY (credential_id) REFERENCES credentials_metadata (id)
    )",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_download_jobs_status
     ON download_jobs(status, updated_at)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_download_jobs_credential
     ON download_jobs(credential_id)",
    [],
)?;

// Create download_items table
conn.execute(
    "CREATE TABLE IF NOT EXISTS download_items (
        id VARCHAR PRIMARY KEY,
        job_id VARCHAR NOT NULL,
        source_identifier VARCHAR NOT NULL,
        source_folder VARCHAR,
        item_type VARCHAR NOT NULL,
        status VARCHAR NOT NULL,
        size_bytes BIGINT,
        mime_type VARCHAR,
        metadata VARCHAR,
        error_message VARCHAR,
        retry_count INTEGER DEFAULT 0,
        last_attempt_at BIGINT,
        local_path VARCHAR,
        created_at BIGINT NOT NULL,
        updated_at BIGINT NOT NULL,
        downloaded_at BIGINT,
        FOREIGN KEY (job_id) REFERENCES download_jobs (id) ON DELETE CASCADE
    )",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_download_items_job_status
     ON download_items(job_id, status)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_download_items_source_identifier
     ON download_items(job_id, source_identifier)",
    [],
)?;
```

#### 1.2 Create Shared Types

**File**: `/Users/brainless/Projects/dwata/shared-types/src/download.rs`

Create the file with all type definitions shown above in "Type Definitions" section.

**File**: `/Users/brainless/Projects/dwata/shared-types/src/lib.rs`

Add:
```rust
pub mod download;

// Add to exports
pub use download::{
    CreateDownloadJobRequest, DownloadJob, DownloadJobListResponse, DownloadJobStatus,
    DownloadProgress, ImapDownloadState, ImapFolderStatus, ImapSyncStrategy, SourceType,
    DownloadItem, DownloadItemStatus,
};
```

### Phase 2: Database Operations

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/database/downloads.rs`

```rust
use duckdb::Connection;
use shared_types::download::{
    CreateDownloadJobRequest, DownloadJob, DownloadJobStatus, DownloadProgress,
    ImapDownloadState, SourceType,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum DownloadDbError {
    NotFound,
    DatabaseError(String),
}

/// Generate unique ID for download jobs
fn generate_job_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_part: String = (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            "abcdefghijklmnopqrstuvwxyz0123456789"
                .chars()
                .nth(idx)
                .unwrap()
        })
        .collect();
    format!("job_{}", random_part)
}

/// Insert new download job
pub async fn insert_download_job(
    conn: Arc<Mutex<Connection>>,
    request: &CreateDownloadJobRequest,
) -> Result<DownloadJob, DownloadDbError> {
    let conn = conn.lock().await;
    let id = generate_job_id();
    let now = chrono::Utc::now().timestamp_millis();

    // Serialize source_config to JSON
    let source_state_json = serde_json::to_string(&request.source_config)
        .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to serialize config: {}", e)))?;

    conn.execute(
        "INSERT INTO download_jobs
         (id, source_type, credential_id, status, source_state, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        duckdb::params![
            &id,
            request.source_type.as_str(),
            &request.credential_id,
            "pending",
            &source_state_json,
            now,
            now,
        ],
    )
    .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(DownloadJob {
        id,
        source_type: request.source_type.clone(),
        credential_id: request.credential_id.clone(),
        status: DownloadJobStatus::Pending,
        progress: DownloadProgress {
            total_items: 0,
            downloaded_items: 0,
            failed_items: 0,
            skipped_items: 0,
            in_progress_items: 0,
            remaining_items: 0,
            percent_complete: 0.0,
            bytes_downloaded: 0,
            items_per_second: 0.0,
            estimated_completion_secs: None,
        },
        source_state: request.source_config.clone(),
        error_message: None,
        created_at: now,
        started_at: None,
        updated_at: now,
        completed_at: None,
        last_sync_at: None,
    })
}

/// Get download job by ID
pub async fn get_download_job(
    conn: Arc<Mutex<Connection>>,
    id: &str,
) -> Result<DownloadJob, DownloadDbError> {
    let conn = conn.lock().await;

    let mut stmt = conn
        .prepare(
            "SELECT id, source_type, credential_id, status, total_items, downloaded_items,
                    failed_items, skipped_items, in_progress_items, bytes_downloaded,
                    source_state, error_message, created_at, started_at, updated_at,
                    completed_at, last_sync_at
             FROM download_jobs
             WHERE id = ?",
        )
        .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    stmt.query_row([id], |row| {
        let source_type_str: String = row.get(1)?;
        let source_type = match source_type_str.as_str() {
            "imap" => SourceType::Imap,
            "google-drive" => SourceType::GoogleDrive,
            "dropbox" => SourceType::Dropbox,
            "onedrive" => SourceType::OneDrive,
            _ => SourceType::Imap,
        };

        let status_str: String = row.get(3)?;
        let status = match status_str.as_str() {
            "pending" => DownloadJobStatus::Pending,
            "running" => DownloadJobStatus::Running,
            "paused" => DownloadJobStatus::Paused,
            "completed" => DownloadJobStatus::Completed,
            "failed" => DownloadJobStatus::Failed,
            "cancelled" => DownloadJobStatus::Cancelled,
            _ => DownloadJobStatus::Pending,
        };

        let total_items: i64 = row.get(4)?;
        let downloaded_items: i64 = row.get(5)?;
        let failed_items: i64 = row.get(6)?;
        let skipped_items: i64 = row.get(7)?;
        let in_progress_items: i64 = row.get(8)?;

        let remaining = total_items.saturating_sub(downloaded_items + failed_items + skipped_items);
        let percent = if total_items > 0 {
            ((downloaded_items + skipped_items) as f32 / total_items as f32) * 100.0
        } else {
            0.0
        };

        let source_state_json: String = row.get(10)?;
        let source_state: serde_json::Value = serde_json::from_str(&source_state_json).unwrap_or(serde_json::json!({}));

        Ok(DownloadJob {
            id: row.get(0)?,
            source_type,
            credential_id: row.get(2)?,
            status,
            progress: DownloadProgress {
                total_items: total_items as u64,
                downloaded_items: downloaded_items as u64,
                failed_items: failed_items as u64,
                skipped_items: skipped_items as u64,
                in_progress_items: in_progress_items as u64,
                remaining_items: remaining as u64,
                percent_complete: percent,
                bytes_downloaded: row.get(9)?,
                items_per_second: 0.0,
                estimated_completion_secs: None,
            },
            source_state,
            error_message: row.get(11)?,
            created_at: row.get(12)?,
            started_at: row.get(13)?,
            updated_at: row.get(14)?,
            completed_at: row.get(15)?,
            last_sync_at: row.get(16)?,
        })
    })
    .map_err(|e| match e {
        duckdb::Error::QueryReturnedNoRows => DownloadDbError::NotFound,
        _ => DownloadDbError::DatabaseError(e.to_string()),
    })
}

/// List download jobs (optionally filter by status)
pub async fn list_download_jobs(
    conn: Arc<Mutex<Connection>>,
    status_filter: Option<&str>,
    limit: usize,
) -> Result<Vec<DownloadJob>, DownloadDbError> {
    let conn = conn.lock().await;

    let query = if let Some(status) = status_filter {
        format!(
            "SELECT id FROM download_jobs WHERE status = '{}' ORDER BY created_at DESC LIMIT {}",
            status, limit
        )
    } else {
        format!("SELECT id FROM download_jobs ORDER BY created_at DESC LIMIT {}", limit)
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    let mut jobs = Vec::new();
    for id_result in ids {
        let id = id_result.map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;
        drop(conn);
        if let Ok(job) = get_download_job(conn.clone(), &id).await {
            jobs.push(job);
        }
    }

    Ok(jobs)
}

/// Update job status
pub async fn update_job_status(
    conn: Arc<Mutex<Connection>>,
    job_id: &str,
    status: DownloadJobStatus,
    error_message: Option<String>,
) -> Result<(), DownloadDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let status_str = match status {
        DownloadJobStatus::Pending => "pending",
        DownloadJobStatus::Running => "running",
        DownloadJobStatus::Paused => "paused",
        DownloadJobStatus::Completed => "completed",
        DownloadJobStatus::Failed => "failed",
        DownloadJobStatus::Cancelled => "cancelled",
    };

    conn.execute(
        "UPDATE download_jobs
         SET status = ?, error_message = ?, updated_at = ?
         WHERE id = ?",
        duckdb::params![status_str, &error_message, now, job_id],
    )
    .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Update job progress counters
pub async fn update_job_progress(
    conn: Arc<Mutex<Connection>>,
    job_id: &str,
    total_items: Option<u64>,
    downloaded_items: Option<u64>,
    failed_items: Option<u64>,
    skipped_items: Option<u64>,
    bytes_downloaded: Option<u64>,
) -> Result<(), DownloadDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let mut updates = vec!["updated_at = ?"];
    let mut params: Vec<Box<dyn duckdb::ToSql>> = vec![Box::new(now)];

    if let Some(total) = total_items {
        updates.push("total_items = ?");
        params.push(Box::new(total as i64));
    }
    if let Some(downloaded) = downloaded_items {
        updates.push("downloaded_items = ?");
        params.push(Box::new(downloaded as i64));
    }
    if let Some(failed) = failed_items {
        updates.push("failed_items = ?");
        params.push(Box::new(failed as i64));
    }
    if let Some(skipped) = skipped_items {
        updates.push("skipped_items = ?");
        params.push(Box::new(skipped as i64));
    }
    if let Some(bytes) = bytes_downloaded {
        updates.push("bytes_downloaded = ?");
        params.push(Box::new(bytes as i64));
    }

    params.push(Box::new(job_id.to_string()));

    let query = format!(
        "UPDATE download_jobs SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Update source state (e.g., folder checkpoints)
pub async fn update_source_state(
    conn: Arc<Mutex<Connection>>,
    job_id: &str,
    source_state: serde_json::Value,
) -> Result<(), DownloadDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let source_state_json = serde_json::to_string(&source_state)
        .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to serialize state: {}", e)))?;

    conn.execute(
        "UPDATE download_jobs
         SET source_state = ?, updated_at = ?
         WHERE id = ?",
        duckdb::params![&source_state_json, now, job_id],
    )
    .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(())
}
```

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/database/mod.rs`

Add:
```rust
pub mod downloads;
```

### Phase 3: nocodo Integration

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/integrations/nocodo.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::io::Write;

// Re-use nocodo types
use nocodo_tools::types::imap::{
    ImapReaderRequest, ImapOperation, MailboxInfo, MailboxStatusInfo,
    EmailHeader, EmailContent,
};

/// Wrapper for nocodo IMAP tool
pub struct NocodoImapClient {
    config_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct ImapConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
}

impl NocodoImapClient {
    /// Create client with credentials (writes temp config file)
    pub async fn new(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        // Create temporary config file
        let mut temp_file = NamedTempFile::new()?;
        let config = ImapConfig {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: password.to_string(),
        };

        let config_json = serde_json::to_string_pretty(&config)?;
        temp_file.write_all(config_json.as_bytes())?;

        let config_path = temp_file.into_temp_path().keep()?;

        Ok(Self { config_path })
    }

    /// List all mailboxes/folders
    pub async fn list_mailboxes(&self) -> Result<Vec<MailboxInfo>> {
        let request = ImapReaderRequest {
            config_path: Some(self.config_path.to_string_lossy().to_string()),
            operation: ImapOperation::ListMailboxes { pattern: Some("*".to_string()) },
            timeout_seconds: Some(30),
        };

        let response = nocodo_tools::imap::execute_imap_request(request).await?;

        // Parse response (type depends on nocodo response structure)
        // This is a simplified version - adjust based on actual nocodo API
        Ok(vec![])
    }

    /// Get mailbox status (message counts)
    pub async fn mailbox_status(&self, mailbox: &str) -> Result<MailboxStatusInfo> {
        let request = ImapReaderRequest {
            config_path: Some(self.config_path.to_string_lossy().to_string()),
            operation: ImapOperation::MailboxStatus { mailbox: mailbox.to_string() },
            timeout_seconds: Some(30),
        };

        let response = nocodo_tools::imap::execute_imap_request(request).await?;

        // Parse and return
        unimplemented!("Parse nocodo response")
    }

    /// Search for email UIDs
    pub async fn search_emails(
        &self,
        mailbox: &str,
        since_uid: Option<u32>,
        limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        // Build search criteria
        let criteria = nocodo_tools::types::imap::SearchCriteria {
            from: None,
            to: None,
            subject: None,
            since_date: None,
            before_date: None,
            unseen_only: false,
            raw_query: since_uid.map(|uid| format!("UID {}:*", uid)),
        };

        let request = ImapReaderRequest {
            config_path: Some(self.config_path.to_string_lossy().to_string()),
            operation: ImapOperation::Search {
                mailbox: mailbox.to_string(),
                criteria,
                limit,
            },
            timeout_seconds: Some(60),
        };

        let response = nocodo_tools::imap::execute_imap_request(request).await?;

        // Parse UIDs from response
        unimplemented!("Parse nocodo response")
    }

    /// Fetch email headers (lightweight metadata)
    pub async fn fetch_headers(&self, mailbox: &str, uids: &[u32]) -> Result<Vec<EmailHeader>> {
        let request = ImapReaderRequest {
            config_path: Some(self.config_path.to_string_lossy().to_string()),
            operation: ImapOperation::FetchHeaders {
                mailbox: mailbox.to_string(),
                message_uids: uids.to_vec(),
            },
            timeout_seconds: Some(60),
        };

        let response = nocodo_tools::imap::execute_imap_request(request).await?;

        unimplemented!("Parse nocodo response")
    }

    /// Fetch full email content
    pub async fn fetch_email(&self, mailbox: &str, uid: u32) -> Result<EmailContent> {
        let request = ImapReaderRequest {
            config_path: Some(self.config_path.to_string_lossy().to_string()),
            operation: ImapOperation::FetchEmail {
                mailbox: mailbox.to_string(),
                message_uid: uid,
                include_html: Some(true),
                include_text: Some(true),
            },
            timeout_seconds: Some(120),
        };

        let response = nocodo_tools::imap::execute_imap_request(request).await?;

        unimplemented!("Parse nocodo response")
    }
}

impl Drop for NocodoImapClient {
    fn drop(&mut self) {
        // Clean up temp config file
        let _ = std::fs::remove_file(&self.config_path);
    }
}
```

**Note**: The actual implementation will depend on how nocodo's IMAP tool returns responses. Adjust parsing logic accordingly.

### Phase 4: Download Manager

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/jobs/download_manager.rs`

```rust
use crate::database::downloads as db;
use crate::integrations::nocodo::NocodoImapClient;
use anyhow::Result;
use shared_types::download::{DownloadJob, DownloadJobStatus, ImapDownloadState, SourceType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub struct DownloadManager {
    db_conn: Arc<Mutex<duckdb::Connection>>,
    active_jobs: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl DownloadManager {
    pub fn new(db_conn: Arc<Mutex<duckdb::Connection>>) -> Self {
        Self {
            db_conn,
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start or resume a download job
    pub async fn start_job(&self, job_id: &str) -> Result<()> {
        // Get job from database
        let job = db::get_download_job(self.db_conn.clone(), job_id).await?;

        // Check if already running
        if self.active_jobs.lock().await.contains_key(job_id) {
            return Err(anyhow::anyhow!("Job already running"));
        }

        // Update status to running
        db::update_job_status(
            self.db_conn.clone(),
            job_id,
            DownloadJobStatus::Running,
            None,
        )
        .await?;

        // Spawn background task
        let db_conn = self.db_conn.clone();
        let job_id = job_id.to_string();

        let handle = tokio::spawn(async move {
            match job.source_type {
                SourceType::Imap => {
                    if let Err(e) = Self::run_imap_download(db_conn.clone(), &job).await {
                        error!("IMAP download failed for job {}: {}", job_id, e);
                        let _ = db::update_job_status(
                            db_conn,
                            &job_id,
                            DownloadJobStatus::Failed,
                            Some(e.to_string()),
                        )
                        .await;
                    }
                }
                _ => {
                    warn!("Unsupported source type: {:?}", job.source_type);
                }
            }
        });

        self.active_jobs.lock().await.insert(job_id.to_string(), handle);

        Ok(())
    }

    /// Pause a running job
    pub async fn pause_job(&self, job_id: &str) -> Result<()> {
        // Cancel Tokio task
        if let Some(handle) = self.active_jobs.lock().await.remove(job_id) {
            handle.abort();
        }

        // Update status
        db::update_job_status(
            self.db_conn.clone(),
            job_id,
            DownloadJobStatus::Paused,
            None,
        )
        .await?;

        Ok(())
    }

    /// Main IMAP download loop
    async fn run_imap_download(
        db_conn: Arc<Mutex<duckdb::Connection>>,
        job: &DownloadJob,
    ) -> Result<()> {
        info!("Starting IMAP download for job {}", job.id);

        // Get credentials from database
        let credential = crate::database::credentials::get_credential(
            db_conn.clone(),
            &job.credential_id,
        )
        .await?;

        // Get password from keychain
        let password = crate::helpers::keyring_service::KeyringService::get_password(
            &credential.credential_type,
            &credential.identifier,
            &credential.username,
        )?;

        // Create IMAP client
        let imap_client = NocodoImapClient::new(
            &credential.service_name.unwrap_or_default(),
            credential.port.unwrap_or(993) as u16,
            &credential.username,
            &password,
        )
        .await?;

        // Parse IMAP state from source_state
        let state: ImapDownloadState = serde_json::from_value(job.source_state.clone())?;

        // Download each folder
        for folder in &state.folders {
            info!("Processing folder: {}", folder.name);

            // Check mailbox status
            let mailbox_status = imap_client.mailbox_status(&folder.name).await?;

            // Update total if changed
            if mailbox_status.messages.unwrap_or(0) != folder.total_messages {
                info!(
                    "Folder {} message count changed: {} -> {}",
                    folder.name,
                    folder.total_messages,
                    mailbox_status.messages.unwrap_or(0)
                );
                // TODO: Update folder totals in state
            }

            // Search for UIDs to download
            let resume_uid = folder.last_synced_uid;
            let uids = imap_client
                .search_emails(&folder.name, resume_uid, Some(state.fetch_batch_size))
                .await?;

            info!("Found {} emails to download in {}", uids.len(), folder.name);

            // Download in batches
            for batch in uids.chunks(state.fetch_batch_size) {
                // Fetch headers first
                let headers = imap_client.fetch_headers(&folder.name, batch).await?;

                for header in headers {
                    // Download full email
                    match imap_client.fetch_email(&folder.name, header.uid).await {
                        Ok(email) => {
                            // TODO: Store email content in database
                            info!("Downloaded email UID {}", header.uid);

                            // Update progress
                            db::update_job_progress(
                                db_conn.clone(),
                                &job.id,
                                None,
                                Some(1), // Increment downloaded
                                None,
                                None,
                                Some(header.size.unwrap_or(0) as u64),
                            )
                            .await?;
                        }
                        Err(e) => {
                            error!("Failed to download email UID {}: {}", header.uid, e);

                            // Increment failed count
                            db::update_job_progress(
                                db_conn.clone(),
                                &job.id,
                                None,
                                None,
                                Some(1), // Increment failed
                                None,
                                None,
                            )
                            .await?;
                        }
                    }
                }

                // Update checkpoint after each batch
                // TODO: Update folder.last_synced_uid in state
            }
        }

        // Mark job as completed
        db::update_job_status(
            db_conn,
            &job.id,
            DownloadJobStatus::Completed,
            None,
        )
        .await?;

        info!("IMAP download completed for job {}", job.id);
        Ok(())
    }

    /// Periodic sync: check for new emails
    pub async fn sync_all_jobs(&self) -> Result<()> {
        let jobs = db::list_download_jobs(self.db_conn.clone(), None, 100).await?;

        for job in jobs {
            if job.status == DownloadJobStatus::Running || job.status == DownloadJobStatus::Completed {
                // TODO: Check for new items at source
                // If new items found, update total_items and optionally resume
            }
        }

        Ok(())
    }

    /// On server startup, resume interrupted jobs
    pub async fn restore_interrupted_jobs(&self) -> Result<()> {
        let interrupted_jobs = db::list_download_jobs(
            self.db_conn.clone(),
            Some("running"),
            100,
        )
        .await?;

        for job in interrupted_jobs {
            info!("Resuming interrupted job: {}", job.id);
            self.start_job(&job.id).await?;
        }

        Ok(())
    }
}
```

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/jobs/mod.rs`

Create this file:
```rust
pub mod download_manager;
```

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/integrations/mod.rs`

Create this file:
```rust
pub mod nocodo;
```

### Phase 5: API Handlers

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/handlers/downloads.rs`

```rust
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use shared_types::download::{
    CreateDownloadJobRequest, DownloadJob, DownloadJobListResponse, DownloadJobStatus,
};

use crate::database::downloads as db;
use crate::jobs::download_manager::DownloadManager;

/// POST /api/downloads - Create new download job
pub async fn create_download_job(
    db_conn: web::Data<Arc<tokio::sync::Mutex<duckdb::Connection>>>,
    request: web::Json<CreateDownloadJobRequest>,
) -> ActixResult<HttpResponse> {
    let job = db::insert_download_job(db_conn.into_inner().as_ref().clone(), &request)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Created().json(job))
}

/// GET /api/downloads - List all download jobs
#[derive(Deserialize)]
pub struct ListQuery {
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

pub async fn list_download_jobs(
    db_conn: web::Data<Arc<tokio::sync::Mutex<duckdb::Connection>>>,
    query: web::Query<ListQuery>,
) -> ActixResult<HttpResponse> {
    let jobs = db::list_download_jobs(
        db_conn.into_inner().as_ref().clone(),
        query.status.as_deref(),
        query.limit,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(DownloadJobListResponse { jobs }))
}

/// GET /api/downloads/:id - Get download job status
pub async fn get_download_job(
    db_conn: web::Data<Arc<tokio::sync::Mutex<duckdb::Connection>>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    let job = db::get_download_job(db_conn.into_inner().as_ref().clone(), &job_id)
        .await
        .map_err(|e| match e {
            db::DownloadDbError::NotFound => actix_web::error::ErrorNotFound("Job not found"),
            _ => actix_web::error::ErrorInternalServerError(e.to_string()),
        })?;

    Ok(HttpResponse::Ok().json(job))
}

/// POST /api/downloads/:id/start - Start or resume download
pub async fn start_download(
    manager: web::Data<Arc<DownloadManager>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    manager
        .start_job(&job_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "started" })))
}

/// POST /api/downloads/:id/pause - Pause download
pub async fn pause_download(
    manager: web::Data<Arc<DownloadManager>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    manager
        .pause_job(&job_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "paused" })))
}

/// DELETE /api/downloads/:id - Cancel and delete download job
pub async fn delete_download_job(
    db_conn: web::Data<Arc<tokio::sync::Mutex<duckdb::Connection>>>,
    manager: web::Data<Arc<DownloadManager>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    // Pause if running
    let _ = manager.pause_job(&job_id).await;

    // Update status to cancelled
    db::update_job_status(
        db_conn.into_inner().as_ref().clone(),
        &job_id,
        DownloadJobStatus::Cancelled,
        None,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::NoContent().finish())
}
```

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/handlers/mod.rs`

Add:
```rust
pub mod downloads;
```

### Phase 6: Route Registration & Server Setup

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/main.rs`

Add to router:
```rust
use crate::handlers::downloads;
use crate::jobs::download_manager::DownloadManager;

// In main() function, after creating db_conn:
let download_manager = Arc::new(DownloadManager::new(db_conn.clone()));

// Restore interrupted jobs on startup
download_manager.restore_interrupted_jobs().await?;

// Spawn periodic sync task (every 5 minutes)
let manager_clone = download_manager.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
    loop {
        interval.tick().await;
        if let Err(e) = manager_clone.sync_all_jobs().await {
            tracing::error!("Periodic sync failed: {}", e);
        }
    }
});

// Add routes
let app = HttpServer::new(move || {
    App::new()
        // ... existing routes ...
        .route("/api/downloads", web::post().to(downloads::create_download_job))
        .route("/api/downloads", web::get().to(downloads::list_download_jobs))
        .route("/api/downloads/{id}", web::get().to(downloads::get_download_job))
        .route("/api/downloads/{id}/start", web::post().to(downloads::start_download))
        .route("/api/downloads/{id}/pause", web::post().to(downloads::pause_download))
        .route("/api/downloads/{id}", web::delete().to(downloads::delete_download_job))
        .app_data(web::Data::new(db_conn.clone()))
        .app_data(web::Data::new(download_manager.clone()))
})
.bind(("127.0.0.1", 8080))?
.run()
.await
```

### Phase 7: TypeScript Type Generation

Run type generation:
```bash
cd /Users/brainless/Projects/dwata/shared-types
cargo run --bin generate_api_types
```

This will generate TypeScript types in `gui/` for all types marked with `#[ts(export)]`.

## Testing Strategy

### Manual Testing Checklist

1. **Create Download Job**
   ```bash
   curl -X POST http://localhost:8080/api/downloads \
     -H "Content-Type: application/json" \
     -d '{
       "credential_id": "cred_abc123xyz",
       "source_type": "imap",
       "source_config": {
         "folders": [
           {
             "name": "INBOX",
             "total_messages": 0,
             "downloaded_messages": 0,
             "failed_messages": 0,
             "skipped_messages": 0,
             "last_synced_uid": null,
             "is_complete": false
           }
         ],
         "sync_strategy": "inbox-only",
         "last_highest_uid": {},
         "fetch_batch_size": 100
       }
     }'
   ```

2. **List Download Jobs**
   ```bash
   curl http://localhost:8080/api/downloads
   ```

3. **Get Job Status**
   ```bash
   curl http://localhost:8080/api/downloads/job_abc123xyz
   ```

4. **Start Download**
   ```bash
   curl -X POST http://localhost:8080/api/downloads/job_abc123xyz/start
   ```

5. **Pause Download**
   ```bash
   curl -X POST http://localhost:8080/api/downloads/job_abc123xyz/pause
   ```

6. **Monitor Progress**
   - Repeatedly call GET `/api/downloads/:id`
   - Verify `downloaded_items` increases
   - Verify `percent_complete` updates

7. **Test Resumption**
   - Start download
   - Kill server (Ctrl+C)
   - Restart server
   - Verify job status is `running`
   - Verify download continues from checkpoint

8. **Test New Email Detection**
   - Complete a download
   - Send new email to IMAP account
   - Wait 5 minutes (or trigger sync manually)
   - Verify `total_items` increases
   - Verify job can be resumed

### Error Scenario Testing

1. **Invalid Credential ID**
   - Create job with non-existent credential
   - Expected: 400 or 500 error

2. **IMAP Connection Failure**
   - Use incorrect password in credential
   - Start download
   - Expected: Job status becomes `failed`, error_message populated

3. **Network Interruption**
   - Start download
   - Disconnect network mid-download
   - Expected: Job continues from checkpoint after network restores

## Success Criteria

### Implementation Complete When:

1. ✅ Database schema created (download_jobs, download_items)
2. ✅ Shared types created and exported
3. ✅ Database operations implemented
4. ✅ nocodo integration implemented
5. ✅ DownloadManager implemented
6. ✅ API handlers implemented
7. ✅ Routes registered
8. ✅ TypeScript types generated
9. ⚠️  Startup job restoration works (compilation errors need fixing)
10. ⚠️  Periodic sync works (compilation errors need fixing)
11. ⏸  All manual tests pass (pending compilation fixes)

**Note:** See `tasks/fix-download-compilation-errors.md` for remaining work to resolve type mismatches between sync and async database access.

### Quality Checklist:

- [ ] Progress updates after each batch
- [ ] Checkpoints stored for resumption
- [ ] Error handling for network failures
- [ ] Periodic sync detects new emails
- [ ] Multiple jobs can run concurrently
- [ ] Pausing stops download gracefully
- [ ] Cancellation cleans up resources
- [ ] TypeScript types match Rust types
- [ ] No credentials logged
- [ ] Performance acceptable for large mailboxes (1000+ emails)

## Future Enhancements

### Phase 2 Features

1. **Cloud Storage Support**
   - Google Drive file downloads
   - Dropbox file downloads
   - OneDrive file downloads
   - Same progress tracking architecture

2. **Advanced Features**
   - WebSocket for real-time progress updates
   - Email content extraction (save to extraction pipeline)
   - Attachment download options
   - Selective sync (by folder, date range, sender)

3. **Performance Optimizations**
   - Parallel folder downloads
   - Configurable batch sizes
   - Rate limiting to avoid IMAP throttling
   - Compression for stored email content

4. **GUI Improvements**
   - Progress bars per folder
   - Estimated time remaining
   - Download history
   - Retry failed items

## Dependencies

### Cargo.toml Additions

**File**: `/Users/brainless/Projects/dwata/dwata-api/Cargo.toml`

Add:
```toml
[dependencies]
nocodo-tools = { path = "/Users/brainless/Projects/nocodo/nocodo-tools" }
tempfile = "3.8"
```

## Additional Resources

- **nocodo IMAP Tool**: `/Users/brainless/Projects/nocodo/nocodo-tools/src/imap/`
- **nocodo Types**: `/Users/brainless/Projects/nocodo/nocodo-tools/src/types/imap.rs`
- **HackerNews Reference**: `/Users/brainless/Projects/nocodo/nocodo-tools/src/hackernews/` (similar download pattern)

---

**Document Version**: 1.0
**Created**: 2026-01-28
**Status**: Ready for Implementation
