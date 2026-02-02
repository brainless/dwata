use crate::database::AsyncDbConnection;
use anyhow::Result;
use duckdb::params;

pub async fn mark_source_processed(
    conn: AsyncDbConnection,
    source_type: &str,
    source_id: &str,
    extraction_job_id: Option<i64>,
    transaction_count: i32,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let id: i64 = conn.query_row(
        "INSERT INTO financial_extraction_sources
         (source_type, source_id, extraction_job_id, extracted_at, transaction_count)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (source_type, source_id)
         DO UPDATE SET
            extraction_job_id = excluded.extraction_job_id,
            extracted_at = excluded.extracted_at,
            transaction_count = excluded.transaction_count
         RETURNING id",
        params![source_type, source_id, extraction_job_id, now, transaction_count],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn is_source_processed(
    conn: AsyncDbConnection,
    source_type: &str,
    source_id: &str,
) -> Result<bool> {
    let conn = conn.lock().await;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM financial_extraction_sources
         WHERE source_type = ? AND source_id = ?",
        params![source_type, source_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

pub async fn get_processed_sources(
    conn: AsyncDbConnection,
    source_type: &str,
) -> Result<Vec<String>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT source_id FROM financial_extraction_sources
         WHERE source_type = ?",
    )?;

    let rows = stmt.query_map([source_type], |row| row.get::<_, String>(0))?;

    let mut sources = Vec::new();
    for row_result in rows {
        sources.push(row_result?);
    }

    Ok(sources)
}

pub async fn list_processed_sources(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<ProcessedSource>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, source_type, source_id, extraction_job_id, extracted_at, transaction_count
         FROM financial_extraction_sources
         ORDER BY extracted_at DESC
         LIMIT ?",
    )?;

    let rows = stmt.query_map([limit as i64], |row| {
        Ok(ProcessedSource {
            id: row.get(0)?,
            source_type: row.get(1)?,
            source_id: row.get(2)?,
            extraction_job_id: row.get(3)?,
            extracted_at: row.get(4)?,
            transaction_count: row.get(5)?,
        })
    })?;

    let mut sources = Vec::new();
    for row_result in rows {
        sources.push(row_result?);
    }

    Ok(sources)
}

pub async fn delete_processed_source(
    conn: AsyncDbConnection,
    source_type: &str,
    source_id: &str,
) -> Result<()> {
    let conn = conn.lock().await;

    conn.execute(
        "DELETE FROM financial_extraction_sources
         WHERE source_type = ? AND source_id = ?",
        params![source_type, source_id],
    )?;

    Ok(())
}

#[derive(Debug)]
pub struct ProcessedSource {
    pub id: i64,
    pub source_type: String,
    pub source_id: String,
    pub extraction_job_id: Option<i64>,
    pub extracted_at: i64,
    pub transaction_count: i32,
}
