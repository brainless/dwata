use rusqlite::params;
use shared_types::EmailFolder;
use crate::integrations::real_imap_client::FolderMetadata;

use crate::database::AsyncDbConnection;
use anyhow::Result;

pub async fn list_folders(conn: AsyncDbConnection, credential_id: i64) -> Result<Vec<EmailFolder>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, credential_id, name, display_name, imap_path, folder_type, parent_folder_id,
                uidvalidity, last_synced_uid, total_messages, unread_messages, is_subscribed,
                is_selectable, created_at, updated_at, last_synced_at
         FROM email_folders
         WHERE credential_id = ?
         ORDER BY folder_type, name"
    )?;

    let folders = stmt.query_map([credential_id], |row| {
        Ok(EmailFolder {
            id: row.get(0)?,
            credential_id: row.get(1)?,
            name: row.get(2)?,
            display_name: row.get(3)?,
            imap_path: row.get(4)?,
            folder_type: row.get(5)?,
            parent_folder_id: row.get(6)?,
            uidvalidity: row.get::<_, Option<i32>>(7)?.map(|v| v as u32),
            last_synced_uid: row.get::<_, Option<i32>>(8)?.map(|v| v as u32),
            total_messages: row.get(9)?,
            unread_messages: row.get(10)?,
            is_subscribed: row.get(11)?,
            is_selectable: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
            last_synced_at: row.get(15)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(folders)
}

pub async fn get_folder(conn: AsyncDbConnection, folder_id: i64) -> Result<EmailFolder> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, credential_id, name, display_name, imap_path, folder_type, parent_folder_id,
                uidvalidity, last_synced_uid, total_messages, unread_messages, is_subscribed,
                is_selectable, created_at, updated_at, last_synced_at
         FROM email_folders
         WHERE id = ?"
    )?;

    let folder = stmt.query_row([folder_id], |row| {
        Ok(EmailFolder {
            id: row.get(0)?,
            credential_id: row.get(1)?,
            name: row.get(2)?,
            display_name: row.get(3)?,
            imap_path: row.get(4)?,
            folder_type: row.get(5)?,
            parent_folder_id: row.get(6)?,
            uidvalidity: row.get::<_, Option<i32>>(7)?.map(|v| v as u32),
            last_synced_uid: row.get::<_, Option<i32>>(8)?.map(|v| v as u32),
            total_messages: row.get(9)?,
            unread_messages: row.get(10)?,
            is_subscribed: row.get(11)?,
            is_selectable: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
            last_synced_at: row.get(15)?,
        })
    })?;

    Ok(folder)
}

pub async fn upsert_folder(
    conn: AsyncDbConnection,
    credential_id: i64,
    name: &str,
    imap_path: &str,
    display_name: Option<&str>,
    folder_type: Option<&str>,
    parent_folder_id: Option<i64>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let folder_id: i64 = conn.query_row(
        "INSERT INTO email_folders (credential_id, name, imap_path, display_name, folder_type, parent_folder_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(credential_id, imap_path) DO UPDATE SET
             name = excluded.name,
             display_name = excluded.display_name,
             folder_type = excluded.folder_type,
             parent_folder_id = excluded.parent_folder_id,
             updated_at = excluded.updated_at
         RETURNING id",
        params![
            credential_id,
            name,
            imap_path,
            display_name,
            folder_type,
            parent_folder_id,
            now,
            now,
        ],
        |row| row.get(0)
    )?;

    Ok(folder_id)
}

pub async fn upsert_folder_from_imap(
    conn: AsyncDbConnection,
    credential_id: i64,
    name: &str,
    imap_path: &str,
    is_selectable: bool,
    is_subscribed: bool,
    uidvalidity: Option<u32>,
    total_messages: u32,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let folder_id: i64 = conn.query_row(
        "INSERT INTO email_folders (credential_id, name, imap_path, is_selectable, is_subscribed, uidvalidity, total_messages, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(credential_id, imap_path) DO UPDATE SET
             name = excluded.name,
             is_selectable = excluded.is_selectable,
             is_subscribed = excluded.is_subscribed,
             uidvalidity = excluded.uidvalidity,
             total_messages = excluded.total_messages,
             updated_at = excluded.updated_at
         RETURNING id",
        params![
            credential_id,
            name,
            imap_path,
            is_selectable,
            is_subscribed,
            uidvalidity.map(|v| v as i32),
            total_messages as i32,
            now,
            now,
        ],
        |row| row.get(0)
    )?;

    Ok(folder_id)
}

pub async fn update_folder_stats(
    conn: AsyncDbConnection,
    folder_id: i64,
    total: i32,
    unread: i32,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE email_folders SET total_messages = ?, unread_messages = ?, updated_at = ? WHERE id = ?",
        params![total, unread, now, folder_id],
    )?;

    Ok(())
}

pub async fn update_folder_sync_state(
    conn: AsyncDbConnection,
    folder_id: i64,
    uidvalidity: u32,
    last_uid: u32,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE email_folders SET uidvalidity = ?, last_synced_uid = ?, updated_at = ?, last_synced_at = ? WHERE id = ?",
        params![uidvalidity as i32, last_uid as i32, now, now, folder_id],
    )?;

    Ok(())
}

pub async fn list_folders_for_credential(
    conn: AsyncDbConnection,
    credential_id: i64,
) -> Result<Vec<EmailFolder>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, credential_id, name, display_name, imap_path, folder_type, parent_folder_id,
                uidvalidity, last_synced_uid, total_messages, unread_messages, is_subscribed,
                is_selectable, created_at, updated_at, last_synced_at
         FROM email_folders
         WHERE credential_id = ?
         ORDER BY imap_path",
    )?;

    let folders = stmt.query_map([credential_id], |row| {
        Ok(EmailFolder {
            id: row.get(0)?,
            credential_id: row.get(1)?,
            name: row.get(2)?,
            display_name: row.get(3)?,
            imap_path: row.get(4)?,
            folder_type: row.get(5)?,
            parent_folder_id: row.get(6)?,
            uidvalidity: row.get::<_, Option<i32>>(7)?.map(|v| v as u32),
            last_synced_uid: row.get::<_, Option<i32>>(8)?.map(|v| v as u32),
            total_messages: row.get(9)?,
            unread_messages: row.get(10)?,
            is_subscribed: row.get(11)?,
            is_selectable: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
            last_synced_at: row.get(15)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(folders)
}

pub async fn get_folder_by_path(
    conn: AsyncDbConnection,
    credential_id: i64,
    imap_path: &str,
) -> Result<Option<i64>> {
    let conn = conn.lock().await;

    let folder_id: Option<i64> = conn.query_row(
        "SELECT id FROM email_folders WHERE credential_id = ? AND imap_path = ?",
        params![credential_id, imap_path],
        |row| row.get(0)
    ).ok();

    Ok(folder_id)
}

pub async fn sync_folders_from_imap(
    conn: AsyncDbConnection,
    credential_id: i64,
    folders: Vec<FolderMetadata>,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    for folder in folders {
        conn.execute(
            "INSERT INTO email_folders (credential_id, name, imap_path, is_selectable, is_subscribed, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(credential_id, imap_path) DO UPDATE SET
                name = excluded.name,
                is_selectable = excluded.is_selectable,
                is_subscribed = excluded.is_subscribed,
                updated_at = excluded.updated_at",
            params![
                credential_id,
                folder.name,
                folder.imap_path,
                folder.is_selectable,
                folder.is_subscribed,
                now,
                now,
            ],
        )?;
    }

    Ok(())
}

