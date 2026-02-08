use rusqlite::params;
use shared_types::EmailLabel;

use crate::database::AsyncDbConnection;
use anyhow::Result;

pub async fn list_labels(conn: AsyncDbConnection, credential_id: i64) -> Result<Vec<EmailLabel>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, credential_id, name, display_name, label_type, color, message_count, created_at, updated_at
         FROM email_labels
         WHERE credential_id = ?
         ORDER BY label_type, name"
    )?;

    let labels = stmt.query_map([credential_id], |row| {
        Ok(EmailLabel {
            id: row.get(0)?,
            credential_id: row.get(1)?,
            name: row.get(2)?,
            display_name: row.get(3)?,
            label_type: row.get(4)?,
            color: row.get(5)?,
            message_count: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(labels)
}

pub async fn get_label(conn: AsyncDbConnection, label_id: i64) -> Result<EmailLabel> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, credential_id, name, display_name, label_type, color, message_count, created_at, updated_at
         FROM email_labels
         WHERE id = ?"
    )?;

    let label = stmt.query_row([label_id], |row| {
        Ok(EmailLabel {
            id: row.get(0)?,
            credential_id: row.get(1)?,
            name: row.get(2)?,
            display_name: row.get(3)?,
            label_type: row.get(4)?,
            color: row.get(5)?,
            message_count: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;

    Ok(label)
}

pub async fn upsert_label(
    conn: AsyncDbConnection,
    credential_id: i64,
    name: &str,
    display_name: Option<&str>,
    label_type: &str,
    color: Option<&str>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let label_id: i64 = conn.query_row(
        "INSERT INTO email_labels (credential_id, name, display_name, label_type, color, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(credential_id, name) DO UPDATE SET
             display_name = excluded.display_name,
             label_type = excluded.label_type,
             color = excluded.color,
             updated_at = excluded.updated_at
         RETURNING id",
        params![
            credential_id,
            name,
            display_name,
            label_type,
            color,
            now,
            now,
        ],
        |row| row.get(0)
    )?;

    Ok(label_id)
}

pub async fn add_label_to_email(conn: AsyncDbConnection, email_id: i64, label_id: i64) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT OR IGNORE INTO email_label_associations (email_id, label_id, created_at)
         VALUES (?, ?, ?)",
        params![email_id, label_id, now],
    )?;

    Ok(())
}

pub async fn remove_label_from_email(conn: AsyncDbConnection, email_id: i64, label_id: i64) -> Result<()> {
    let conn = conn.lock().await;

    conn.execute(
        "DELETE FROM email_label_associations WHERE email_id = ? AND label_id = ?",
        params![email_id, label_id],
    )?;

    Ok(())
}

pub async fn get_labels_for_email(conn: AsyncDbConnection, email_id: i64) -> Result<Vec<EmailLabel>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT l.id, l.credential_id, l.name, l.display_name, l.label_type, l.color, l.message_count, l.created_at, l.updated_at
         FROM email_labels l
         INNER JOIN email_label_associations ela ON l.id = ela.label_id
         WHERE ela.email_id = ?
         ORDER BY l.name"
    )?;

    let labels = stmt.query_map([email_id], |row| {
        Ok(EmailLabel {
            id: row.get(0)?,
            credential_id: row.get(1)?,
            name: row.get(2)?,
            display_name: row.get(3)?,
            label_type: row.get(4)?,
            color: row.get(5)?,
            message_count: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(labels)
}

pub async fn get_emails_for_label(
    conn: AsyncDbConnection,
    label_id: i64,
    limit: usize,
    offset: usize,
) -> Result<Vec<i64>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT email_id
         FROM email_label_associations
         WHERE label_id = ?
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?"
    )?;

    let email_ids = stmt.query_map([label_id, limit as i64, offset as i64], |row| {
        Ok(row.get::<_, i64>(0)?)
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(email_ids)
}
