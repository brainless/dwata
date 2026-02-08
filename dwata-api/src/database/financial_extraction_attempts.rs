use crate::database::AsyncDbConnection;
use anyhow::Result;
use rusqlite::params;
use shared_types::FinancialExtractionAttempt;

pub async fn insert_attempt(
    conn: AsyncDbConnection,
    source_type: &str,
    source_account_id: i64,
    attempted_at: i64,
    total_items_scanned: i64,
    transactions_extracted: i64,
    status: &str,
    error_message: Option<&str>,
) -> Result<i64> {
    let conn = conn.lock().await;

    let id: i64 = conn.query_row(
        "INSERT INTO financial_extraction_attempts
         (source_type, source_account_id, attempted_at, total_items_scanned, transactions_extracted, status, error_message)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        params![
            source_type,
            source_account_id,
            attempted_at,
            total_items_scanned,
            transactions_extracted,
            status,
            error_message,
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn list_attempts(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<FinancialExtractionAttempt>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, source_type, source_account_id, attempted_at, total_items_scanned,
                transactions_extracted, status, error_message
         FROM financial_extraction_attempts
         ORDER BY attempted_at DESC
         LIMIT ?",
    )?;

    let rows = stmt.query_map([limit as i64], |row| {
        Ok(FinancialExtractionAttempt {
            id: row.get(0)?,
            source_type: row.get(1)?,
            source_account_id: row.get(2)?,
            attempted_at: row.get(3)?,
            total_items_scanned: row.get(4)?,
            transactions_extracted: row.get(5)?,
            status: row.get(6)?,
            error_message: row.get(7)?,
        })
    })?;

    let mut attempts = Vec::new();
    for row_result in rows {
        attempts.push(row_result?);
    }

    Ok(attempts)
}
