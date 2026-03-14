use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::download::EmailSyncSettings;

pub async fn get_or_create_settings(
    conn: AsyncDbConnection,
    credential_id: i64,
) -> Result<EmailSyncSettings> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let is_paused: i64 = conn.query_row(
        "INSERT INTO email_sync_settings (credential_id, is_paused, created_at, updated_at)
         VALUES (?, 0, ?, ?)
         ON CONFLICT(credential_id) DO UPDATE SET updated_at = excluded.updated_at
         RETURNING is_paused",
        rusqlite::params![credential_id, now, now],
        |row| row.get(0),
    )?;

    Ok(EmailSyncSettings {
        credential_id,
        is_paused: is_paused != 0,
    })
}

pub async fn set_paused(
    conn: AsyncDbConnection,
    credential_id: i64,
    is_paused: bool,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();
    let flag = if is_paused { 1i64 } else { 0i64 };

    conn.execute(
        "INSERT INTO email_sync_settings (credential_id, is_paused, created_at, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(credential_id) DO UPDATE SET is_paused = excluded.is_paused, updated_at = excluded.updated_at",
        rusqlite::params![credential_id, flag, now, now],
    )?;

    Ok(())
}

pub async fn list_active_credential_ids(conn: AsyncDbConnection) -> Result<Vec<i64>> {
    let conn = conn.lock().await;

    let mut stmt =
        conn.prepare("SELECT credential_id FROM email_sync_settings WHERE is_paused = 0")?;

    let ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(ids)
}
