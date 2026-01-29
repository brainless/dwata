use shared_types::download::{
    CreateDownloadJobRequest, DownloadJob, DownloadJobStatus, DownloadProgress,
    SourceType,
};
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

    conn.execute(
        "DELETE FROM download_jobs WHERE id = ?",
        [job_id],
    )
    .map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(())
}

fn source_type_to_string(source_type: &SourceType) -> &'static str {
    match source_type {
        SourceType::Imap => "imap",
        SourceType::GoogleDrive => "google-drive",
        SourceType::Dropbox => "dropbox",
        SourceType::OneDrive => "onedrive",
    }
}
