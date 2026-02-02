use shared_types::download::{
    CreateDownloadJobRequest, DownloadJob, DownloadJobStatus, DownloadProgress,
    SourceType,
};
use shared_types::email::EmailAddress;
use std::fmt;

pub use crate::database::AsyncDbConnection;

#[derive(Debug)]
pub enum DownloadDbError {
    NotFound,
    DatabaseError(String),
}

impl fmt::Display for DownloadDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadDbError::NotFound => write!(f, "Download job not found"),
            DownloadDbError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for DownloadDbError {}

pub async fn insert_download_job(
    conn: AsyncDbConnection,
    request: &CreateDownloadJobRequest,
) -> Result<DownloadJob, DownloadDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let source_state_json = serde_json::to_string(&request.source_config)
        .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to serialize config: {}", e)))?;

    let id: i64 = conn
        .query_row(
            "INSERT INTO download_jobs
             (source_type, credential_id, status, source_state, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING id",
            duckdb::params![
                source_type_to_string(&request.source_type),
                &request.credential_id,
                "pending",
                &source_state_json,
                now,
                now,
            ],
            |row| row.get(0),
        )
        .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(DownloadJob {
        id,
        source_type: request.source_type.clone(),
        credential_id: request.credential_id,
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

pub async fn get_download_job(
    conn: AsyncDbConnection,
    id: i64,
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

pub async fn list_download_jobs(
    conn: AsyncDbConnection,
    status_filter: Option<&str>,
    limit: usize,
) -> Result<Vec<DownloadJob>, DownloadDbError> {
    let conn_ref = conn.clone();

    // Collect all IDs first, then drop the lock
    let ids = {
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

        let ids_result: Result<Vec<i64>, duckdb::Error> = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?
            .collect();

        ids_result.map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?
    }; // Lock is dropped here

    let mut jobs = Vec::new();
    for id in ids {
        if let Ok(job) = get_download_job(conn_ref.clone(), id).await {
            jobs.push(job);
        }
    }

    Ok(jobs)
}

pub async fn update_job_status(
    conn: AsyncDbConnection,
    job_id: i64,
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

pub async fn update_job_progress(
    conn: AsyncDbConnection,
    job_id: i64,
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

    params.push(Box::new(job_id));

    let query = format!(
        "UPDATE download_jobs SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub async fn update_source_state(
    conn: AsyncDbConnection,
    job_id: i64,
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

pub async fn delete_download_job(
    conn: AsyncDbConnection,
    job_id: i64,
) -> Result<(), DownloadDbError> {
    let conn = conn.lock().await;

    // First delete all download_items for this job
    conn.execute(
        "DELETE FROM download_items WHERE job_id = ?",
        [job_id],
    )
    .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    // Then delete the job itself
    conn.execute(
        "DELETE FROM download_jobs WHERE id = ?",
        [job_id],
    )
    .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Insert download item
pub async fn insert_download_item(
    conn: AsyncDbConnection,
    job_id: i64,
    source_identifier: &str,
    source_folder: Option<&str>,
    item_type: &str,
    status: &str,
    size_bytes: Option<i64>,
    mime_type: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> Result<i64, DownloadDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let metadata_json = metadata.map(|m| serde_json::to_string(&m).ok()).flatten();

    let item_id: i64 = conn.query_row(
        "INSERT INTO download_items
         (job_id, source_identifier, source_folder, item_type, status, size_bytes,
          mime_type, metadata, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        duckdb::params![
            job_id as i32,
            source_identifier,
            source_folder,
            item_type,
            status,
            size_bytes.map(|s| s as i32),
            mime_type,
            metadata_json.as_deref(),
            now,
            now
        ],
        |row| row.get(0)
    ).map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(item_id)
}

/// Insert email download with transaction support
/// This ensures download_item, email record, and progress update are atomic
#[allow(clippy::too_many_arguments)]
pub async fn insert_email_download_transactional(
    conn: AsyncDbConnection,
    job_id: i64,
    credential_id: i64,
    uid: u32,
    folder: &str,
    message_id: Option<&str>,
    subject: Option<&str>,
    from_address: &str,
    from_name: Option<&str>,
    to_addresses: &[EmailAddress],
    cc_addresses: &[EmailAddress],
    bcc_addresses: &[EmailAddress],
    reply_to: Option<&str>,
    date_sent: Option<i64>,
    date_received: i64,
    body_text: Option<&str>,
    body_html: Option<&str>,
    is_read: bool,
    is_flagged: bool,
    is_draft: bool,
    is_answered: bool,
    has_attachments: bool,
    attachment_count: i32,
    size_bytes: Option<i32>,
    labels: &[String],
) -> Result<(i64, i64), DownloadDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    // Begin transaction
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to begin transaction: {}", e)))?;

    let result = (|| -> Result<(i64, i64), DownloadDbError> {
        // 1. Insert download_item
        let metadata_json: Option<String> = None;
        let download_item_id: i64 = conn.query_row(
            "INSERT INTO download_items
             (job_id, source_identifier, source_folder, item_type, status, size_bytes,
              mime_type, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            duckdb::params![
                job_id as i32,
                &uid.to_string(),
                Some(folder),
                "email",
                "completed",
                size_bytes,
                Some("message/rfc822"),
                metadata_json.as_deref(),
                now,
                now
            ],
            |row| row.get(0)
        ).map_err(|e| DownloadDbError::DatabaseError(format!("Failed to insert download_item: {}", e)))?;

        // 2. Insert email
        let to_json = serde_json::to_string(to_addresses)
            .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to serialize to_addresses: {}", e)))?;
        let cc_json = serde_json::to_string(cc_addresses)
            .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to serialize cc_addresses: {}", e)))?;
        let bcc_json = serde_json::to_string(bcc_addresses)
            .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to serialize bcc_addresses: {}", e)))?;
        let labels_json = serde_json::to_string(labels)
            .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to serialize labels: {}", e)))?;

        let email_id: i64 = conn.query_row(
            "INSERT INTO emails
             (download_item_id, credential_id, uid, folder, message_id, subject, from_address, from_name,
              to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
              body_text, body_html, is_read, is_flagged, is_draft, is_answered,
              has_attachments, attachment_count, size_bytes, labels, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            duckdb::params![
                download_item_id,
                credential_id,
                uid as i32,
                folder,
                message_id,
                subject,
                from_address,
                from_name,
                &to_json,
                &cc_json,
                &bcc_json,
                reply_to,
                date_sent,
                date_received,
                body_text,
                body_html,
                is_read,
                is_flagged,
                is_draft,
                is_answered,
                has_attachments,
                attachment_count,
                size_bytes,
                &labels_json,
                now,
                now
            ],
            |row| row.get(0)
        ).map_err(|e| DownloadDbError::DatabaseError(format!("Failed to insert email: {}", e)))?;

        // 3. Update job progress (increment downloaded_items by 1)
        conn.execute(
            "UPDATE download_jobs
             SET downloaded_items = downloaded_items + 1,
                 bytes_downloaded = bytes_downloaded + ?,
                 updated_at = ?
             WHERE id = ?",
            duckdb::params![
                size_bytes.unwrap_or(0) as i64,
                now,
                job_id
            ],
        ).map_err(|e| DownloadDbError::DatabaseError(format!("Failed to update progress: {}", e)))?;

        Ok((download_item_id, email_id))
    })();

    match result {
        Ok(ids) => {
            // Commit transaction
            conn.execute("COMMIT", [])
                .map_err(|e| DownloadDbError::DatabaseError(format!("Failed to commit transaction: {}", e)))?;
            Ok(ids)
        }
        Err(e) => {
            // Rollback transaction
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}

fn source_type_to_string(source_type: &SourceType) -> &'static str {
    match source_type {
        SourceType::Imap => "imap",
        SourceType::GoogleDrive => "google-drive",
        SourceType::Dropbox => "dropbox",
        SourceType::OneDrive => "onedrive",
    }
}
