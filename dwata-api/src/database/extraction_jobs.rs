use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::extraction_job::{
    CreateExtractionJobRequest, ExtractionJob, ExtractionJobStatus, ExtractionProgress,
    ExtractionSourceType, ExtractorType,
};

pub async fn insert_extraction_job(
    conn: AsyncDbConnection,
    request: &CreateExtractionJobRequest,
) -> Result<ExtractionJob> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let source_config_json = serde_json::to_string(&request.source_config)?;

    let source_type_str = match request.source_type {
        ExtractionSourceType::EmailAttachment => "email-attachment",
        ExtractionSourceType::LocalFile => "local-file",
        ExtractionSourceType::EmailBody => "email-body",
    };

    let extractor_type_str = match request.extractor_type {
        ExtractorType::AttachmentParser => "attachment-parser",
        ExtractorType::GlinerNER => "gliner-ner",
        ExtractorType::LLMBased => "llm-based",
    };

    let id: i64 = conn.query_row(
        "INSERT INTO extraction_jobs
         (source_type, extractor_type, status, source_config, created_at, updated_at)
         VALUES (?, ?, 'pending', ?, ?, ?)
         RETURNING id",
        duckdb::params![source_type_str, extractor_type_str, &source_config_json, now, now],
        |row| row.get(0),
    )?;

    Ok(ExtractionJob {
        id,
        source_type: request.source_type.clone(),
        extractor_type: request.extractor_type.clone(),
        status: ExtractionJobStatus::Pending,
        progress: ExtractionProgress {
            total_items: 0,
            processed_items: 0,
            extracted_entities: 0,
            failed_items: 0,
            events_extracted: 0,
            contacts_extracted: 0,
            percent_complete: 0.0,
        },
        source_config: serde_json::to_value(&request.source_config)?,
        error_message: None,
        created_at: now,
        started_at: None,
        updated_at: now,
        completed_at: None,
    })
}

pub async fn get_extraction_job(conn: AsyncDbConnection, id: i64) -> Result<ExtractionJob> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, source_type, extractor_type, status, total_items, processed_items,
                extracted_entities, failed_items, events_extracted, contacts_extracted,
                source_config, error_message, created_at, started_at, updated_at, completed_at
         FROM extraction_jobs
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        let source_type_str: String = row.get(1)?;
        let source_type = match source_type_str.as_str() {
            "email-attachment" => ExtractionSourceType::EmailAttachment,
            "local-file" => ExtractionSourceType::LocalFile,
            "email-body" => ExtractionSourceType::EmailBody,
            _ => ExtractionSourceType::EmailAttachment,
        };

        let extractor_type_str: String = row.get(2)?;
        let extractor_type = match extractor_type_str.as_str() {
            "attachment-parser" => ExtractorType::AttachmentParser,
            "gliner-ner" => ExtractorType::GlinerNER,
            "llm-based" => ExtractorType::LLMBased,
            _ => ExtractorType::AttachmentParser,
        };

        let status_str: String = row.get(3)?;
        let status = match status_str.as_str() {
            "pending" => ExtractionJobStatus::Pending,
            "running" => ExtractionJobStatus::Running,
            "completed" => ExtractionJobStatus::Completed,
            "failed" => ExtractionJobStatus::Failed,
            "cancelled" => ExtractionJobStatus::Cancelled,
            _ => ExtractionJobStatus::Pending,
        };

        let total_items: i64 = row.get(4)?;
        let processed_items: i64 = row.get(5)?;
        let extracted_entities: i64 = row.get(6)?;
        let failed_items: i64 = row.get(7)?;
        let events_extracted: i64 = row.get(8)?;
        let contacts_extracted: i64 = row.get(9)?;

        let percent = if total_items > 0 {
            (processed_items as f32 / total_items as f32) * 100.0
        } else {
            0.0
        };

        let source_config_json: String = row.get(10)?;
        let source_config: serde_json::Value =
            serde_json::from_str(&source_config_json).unwrap_or(serde_json::json!({}));

        Ok(ExtractionJob {
            id: row.get(0)?,
            source_type,
            extractor_type,
            status,
            progress: ExtractionProgress {
                total_items: total_items as u64,
                processed_items: processed_items as u64,
                extracted_entities: extracted_entities as u64,
                failed_items: failed_items as u64,
                events_extracted: events_extracted as u64,
                contacts_extracted: contacts_extracted as u64,
                percent_complete: percent,
            },
            source_config,
            error_message: row.get(11)?,
            created_at: row.get(12)?,
            started_at: row.get(13)?,
            updated_at: row.get(14)?,
            completed_at: row.get(15)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get extraction job: {}", e))
}

pub async fn list_extraction_jobs(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<ExtractionJob>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare(
        "SELECT id FROM extraction_jobs ORDER BY created_at DESC LIMIT ?",
    )?;

    let ids: Vec<i64> = stmt.query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut jobs = Vec::new();
    for id in ids {
        if let Ok(job) = get_extraction_job(conn.clone(), id).await {
            jobs.push(job);
        }
    }

    Ok(jobs)
}

pub async fn update_job_status(
    conn: AsyncDbConnection,
    job_id: i64,
    status: ExtractionJobStatus,
    error_message: Option<String>,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let status_str = match status {
        ExtractionJobStatus::Pending => "pending",
        ExtractionJobStatus::Running => "running",
        ExtractionJobStatus::Completed => "completed",
        ExtractionJobStatus::Failed => "failed",
        ExtractionJobStatus::Cancelled => "cancelled",
    };

    conn.execute(
        "UPDATE extraction_jobs
         SET status = ?, error_message = ?, updated_at = ?
         WHERE id = ?",
        duckdb::params![status_str, &error_message, now, job_id],
    )?;

    Ok(())
}

pub async fn update_job_progress(
    conn: AsyncDbConnection,
    job_id: i64,
    total_items: Option<u64>,
    processed_items: Option<u64>,
    events_extracted: Option<u64>,
    contacts_extracted: Option<u64>,
    failed_items: Option<u64>,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let mut updates = vec!["updated_at = ?"];
    let mut params: Vec<Box<dyn duckdb::ToSql>> = vec![Box::new(now)];

    if let Some(total) = total_items {
        updates.push("total_items = ?");
        params.push(Box::new(total as i64));
    }
    if let Some(processed) = processed_items {
        updates.push("processed_items = ?");
        params.push(Box::new(processed as i64));
    }
    if let Some(events) = events_extracted {
        updates.push("events_extracted = ?");
        params.push(Box::new(events as i64));
    }
    if let Some(contacts) = contacts_extracted {
        updates.push("contacts_extracted = ?");
        params.push(Box::new(contacts as i64));
    }
    if let Some(failed) = failed_items {
        updates.push("failed_items = ?");
        params.push(Box::new(failed as i64));
    }

    if events_extracted.is_some() || contacts_extracted.is_some() {
        updates.push("extracted_entities = events_extracted + contacts_extracted");
    }

    params.push(Box::new(job_id));

    let query = format!(
        "UPDATE extraction_jobs SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    conn.execute(&query, params_refs.as_slice())?;

    Ok(())
}
