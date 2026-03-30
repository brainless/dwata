use anyhow::Result;
use rusqlite::params;

use crate::database::AsyncDbConnection;

/// Get unprocessed emails for a credential (no extracted entities yet)
/// Returns emails ordered by date_received DESC (most recent first)
pub async fn get_unprocessed_emails(
    conn: AsyncDbConnection,
    credential_id: i64,
    limit: usize,
) -> Result<Vec<i64>> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.get_blocking();

        // Find emails that don't have any extracted entities
        // We check all entity tables that have source_email_id
        let mut stmt = conn.prepare(
            "SELECT e.id 
             FROM emails e
             WHERE e.credential_id = ?
               AND NOT EXISTS (SELECT 1 FROM subscriptions s WHERE s.source_email_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM bills b WHERE b.source_email_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM transactions t WHERE t.source_email_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM orders o WHERE o.source_email_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM events ev WHERE ev.source_email_id = e.id)
             ORDER BY e.date_received DESC
             LIMIT ?",
        )?;

        let email_ids: Vec<i64> = stmt
            .query_map(params![credential_id, limit as i64], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(email_ids)
    })
    .await?
}

/// Count unprocessed emails for a credential
pub async fn count_unprocessed_emails(conn: AsyncDbConnection, credential_id: i64) -> Result<i64> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.get_blocking();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) 
             FROM emails e
             WHERE e.credential_id = ?
               AND NOT EXISTS (SELECT 1 FROM subscriptions s WHERE s.source_email_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM bills b WHERE b.source_email_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM transactions t WHERE t.source_email_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM orders o WHERE o.source_email_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM events ev WHERE ev.source_email_id = e.id)",
            params![credential_id],
            |row| row.get(0),
        )?;

        Ok(count)
    })
    .await?
}

/// Get total email count for a credential
pub async fn count_total_emails(conn: AsyncDbConnection, credential_id: i64) -> Result<i64> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.get_blocking();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM emails WHERE credential_id = ?",
            params![credential_id],
            |row| row.get(0),
        )?;

        Ok(count)
    })
    .await?
}

/// Get emails for a specific account with pagination
pub async fn get_emails_for_account_paginated(
    conn: AsyncDbConnection,
    credential_id: i64,
    offset: usize,
    limit: usize,
) -> Result<Vec<i64>> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.get_blocking();

        let mut stmt = conn.prepare(
            "SELECT id FROM emails 
             WHERE credential_id = ? 
             ORDER BY date_received DESC 
             LIMIT ? OFFSET ?",
        )?;

        let email_ids: Vec<i64> = stmt
            .query_map(params![credential_id, limit as i64, offset as i64], |row| {
                row.get(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(email_ids)
    })
    .await?
}
