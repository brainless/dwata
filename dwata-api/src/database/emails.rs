use crate::database::AsyncDbConnection;
use anyhow::Result;
use rusqlite::params_from_iter;
use rusqlite::types::Value;
use shared_types::email::{AttachmentExtractionStatus, Email, EmailAddress, EmailAttachment};
use std::collections::HashSet;
use tokio::task;

#[derive(Debug, Clone)]
pub struct EmailScanRow {
    pub email_id: i64,
    pub date_received: i64,
    pub from_address: String,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TemplateCandidateEmailRow {
    pub email_id: i64,
    pub from_address: String,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
}

/// Insert email into database
pub async fn insert_email(
    conn: AsyncDbConnection,
    credential_id: i64,
    download_item_id: Option<i64>,
    uid: u32,
    folder_id: i64,
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
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let to_json = serde_json::to_string(to_addresses)?;
    let cc_json = serde_json::to_string(cc_addresses)?;
    let bcc_json = serde_json::to_string(bcc_addresses)?;

    let email_id: i64 = conn.query_row(
        "INSERT INTO emails
         (download_item_id, credential_id, uid, folder_id, message_id, subject, from_address, from_name,
           to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
           body_text, body_html, is_read, is_flagged, is_draft, is_answered,
           has_attachments, attachment_count, size_bytes, thread_id, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(credential_id, folder_id, uid) DO UPDATE SET
             updated_at = excluded.updated_at
           RETURNING id",
        rusqlite::params![
            download_item_id, credential_id, uid as i32, folder_id, message_id, subject, from_address, from_name,
            &to_json, &cc_json, &bcc_json, reply_to, date_sent, date_received,
            body_text, body_html, is_read, is_flagged, is_draft, is_answered,
            has_attachments, attachment_count, size_bytes, None::<String>, now, now
        ],
        |row| row.get(0)
    )?;

    Ok(email_id)
}

/// Filter out UIDs that already exist for a credential + folder
pub async fn filter_new_uids(
    conn: AsyncDbConnection,
    credential_id: i64,
    folder_id: i64,
    uids: &[u32],
) -> Result<Vec<u32>> {
    if uids.is_empty() {
        return Ok(Vec::new());
    }

    let uids = uids.to_vec();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut existing = HashSet::new();
        let chunk_size = 900; // SQLite max variables is 999

        for chunk in uids.chunks(chunk_size) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");

            let query = format!(
                "SELECT uid FROM emails
                 WHERE credential_id = ? AND folder_id = ? AND uid IN ({})",
                placeholders
            );

            let mut params: Vec<rusqlite::types::Value> = Vec::with_capacity(2 + chunk.len());
            params.push(rusqlite::types::Value::from(credential_id));
            params.push(rusqlite::types::Value::from(folder_id));
            params.extend(
                chunk
                    .iter()
                    .map(|uid| rusqlite::types::Value::from(*uid as i64)),
            );

            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map(params_from_iter(params), |row| row.get::<_, i32>(0))?;
            for row in rows {
                existing.insert(row? as u32);
            }
        }

        let filtered = uids
            .iter()
            .copied()
            .filter(|uid| !existing.contains(uid))
            .collect();

        Ok(filtered)
    })
    .await?
}

/// Get the oldest UID already stored for a folder
pub async fn get_oldest_uid_for_folder(
    conn: AsyncDbConnection,
    credential_id: i64,
    folder_id: i64,
) -> Result<Option<u32>> {
    let conn = conn.lock().await;
    let uid: Option<i32> = conn
        .query_row(
            "SELECT MIN(uid) FROM emails WHERE credential_id = ? AND folder_id = ?",
            [credential_id, folder_id],
            |row| row.get(0),
        )
        .unwrap_or(None);

    Ok(uid.map(|v| v as u32))
}

/// Get email by ID
pub async fn get_email(conn: AsyncDbConnection, email_id: i64) -> Result<Email> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut stmt = conn.prepare(
            "SELECT id, download_item_id, credential_id, uid, folder_id, message_id, subject, from_address, from_name,
                    to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
                    body_text, body_html, is_read, is_flagged, is_draft, is_answered,
                    has_attachments, attachment_count, size_bytes, thread_id,
                    created_at, updated_at
             FROM emails WHERE id = ?"
        )?;

        let email = stmt.query_row([email_id], |row| {
            let to_json: String = row.get(9)?;
            let cc_json: String = row.get(10)?;
            let bcc_json: String = row.get(11)?;

            Ok(Email {
                id: row.get(0)?,
                download_item_id: row.get(1)?,
                credential_id: row.get(2)?,
                uid: row.get::<_, i32>(3)? as u32,
                folder_id: row.get(4)?,
                message_id: row.get(5)?,
                subject: row.get(6)?,
                from_address: row.get(7)?,
                from_name: row.get(8)?,
                to_addresses: serde_json::from_str(&to_json).unwrap_or_default(),
                cc_addresses: serde_json::from_str(&cc_json).unwrap_or_default(),
                bcc_addresses: serde_json::from_str(&bcc_json).unwrap_or_default(),
                reply_to: row.get(12)?,
                date_sent: row.get(13)?,
                date_received: row.get(14)?,
                body_text: row.get(15)?,
                body_html: row.get(16)?,
                is_read: row.get(17)?,
                is_flagged: row.get(18)?,
                is_draft: row.get(19)?,
                is_answered: row.get(20)?,
                has_attachments: row.get(21)?,
                attachment_count: row.get(22)?,
                size_bytes: row.get(23)?,
                thread_id: row.get(24)?,
                created_at: row.get(25)?,
                updated_at: row.get(26)?,
            })
        })?;

        Ok(email)
    })
    .await?
}

/// Get emails by IDs preserving input order
pub async fn list_emails_by_ids(conn: AsyncDbConnection, email_ids: &[i64]) -> Result<Vec<Email>> {
    if email_ids.is_empty() {
        return Ok(Vec::new());
    }

    let email_ids = email_ids.to_vec();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut by_id = std::collections::HashMap::new();

        for chunk in email_ids.chunks(900) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT id, download_item_id, credential_id, uid, folder_id, message_id, subject, from_address, from_name,
                        to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
                        body_text, body_html, is_read, is_flagged, is_draft, is_answered,
                        has_attachments, attachment_count, size_bytes, thread_id,
                        created_at, updated_at
                 FROM emails
                 WHERE id IN ({})",
                placeholders
            );

            let params: Vec<Value> = chunk.iter().copied().map(Value::from).collect();
            let mut stmt = conn.prepare(&query)?;
            let mapped = stmt.query_map(params_from_iter(params), |row| {
                let to_json: String = row.get(9)?;
                let cc_json: String = row.get(10)?;
                let bcc_json: String = row.get(11)?;

                Ok(Email {
                    id: row.get(0)?,
                    download_item_id: row.get(1)?,
                    credential_id: row.get(2)?,
                    uid: row.get::<_, i32>(3)? as u32,
                    folder_id: row.get(4)?,
                    message_id: row.get(5)?,
                    subject: row.get(6)?,
                    from_address: row.get(7)?,
                    from_name: row.get(8)?,
                    to_addresses: serde_json::from_str(&to_json).unwrap_or_default(),
                    cc_addresses: serde_json::from_str(&cc_json).unwrap_or_default(),
                    bcc_addresses: serde_json::from_str(&bcc_json).unwrap_or_default(),
                    reply_to: row.get(12)?,
                    date_sent: row.get(13)?,
                    date_received: row.get(14)?,
                    body_text: row.get(15)?,
                    body_html: row.get(16)?,
                    is_read: row.get(17)?,
                    is_flagged: row.get(18)?,
                    is_draft: row.get(19)?,
                    is_answered: row.get(20)?,
                    has_attachments: row.get(21)?,
                    attachment_count: row.get(22)?,
                    size_bytes: row.get(23)?,
                    thread_id: row.get(24)?,
                    created_at: row.get(25)?,
                    updated_at: row.get(26)?,
                })
            })?;

            for row in mapped {
                let email = row?;
                by_id.insert(email.id, email);
            }
        }

        let mut ordered = Vec::with_capacity(email_ids.len());
        for id in email_ids {
            if let Some(email) = by_id.remove(&id) {
                ordered.push(email);
            }
        }
        Ok(ordered)
    })
    .await?
}

/// List emails with pagination
pub async fn list_emails(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
    folder_id: Option<i64>,
    limit: usize,
    offset: usize,
) -> Result<Vec<Email>> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let query = match (credential_id, folder_id) {
            (Some(cred), Some(fid)) => {
                format!(
                    "SELECT id, download_item_id, credential_id, uid, folder_id, message_id, subject, from_address, from_name,
                            to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
                            body_text, body_html, is_read, is_flagged, is_draft, is_answered,
                            has_attachments, attachment_count, size_bytes, thread_id,
                            created_at, updated_at
                     FROM emails WHERE credential_id = {} AND folder_id = {}
                     ORDER BY date_received DESC LIMIT {} OFFSET {}",
                    cred, fid, limit, offset
                )
            }
            (Some(cred), None) => {
                format!(
                    "SELECT id, download_item_id, credential_id, uid, folder_id, message_id, subject, from_address, from_name,
                            to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
                            body_text, body_html, is_read, is_flagged, is_draft, is_answered,
                            has_attachments, attachment_count, size_bytes, thread_id,
                            created_at, updated_at
                     FROM emails WHERE credential_id = {}
                     ORDER BY date_received DESC LIMIT {} OFFSET {}",
                    cred, limit, offset
                )
            }
            (None, Some(fid)) => {
                format!(
                    "SELECT id, download_item_id, credential_id, uid, folder_id, message_id, subject, from_address, from_name,
                            to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
                            body_text, body_html, is_read, is_flagged, is_draft, is_answered,
                            has_attachments, attachment_count, size_bytes, thread_id,
                            created_at, updated_at
                     FROM emails WHERE folder_id = {}
                     ORDER BY date_received DESC LIMIT {} OFFSET {}",
                    fid, limit, offset
                )
            }
            (None, None) => {
                format!(
                    "SELECT id, download_item_id, credential_id, uid, folder_id, message_id, subject, from_address, from_name,
                            to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
                            body_text, body_html, is_read, is_flagged, is_draft, is_answered,
                            has_attachments, attachment_count, size_bytes, thread_id,
                            created_at, updated_at
                     FROM emails ORDER BY date_received DESC
                     LIMIT {} OFFSET {}",
                    limit, offset
                )
            }
        };

        let mut stmt = conn.prepare(&query)?;
        let emails = stmt
            .query_map([], |row| {
                let to_json: String = row.get(9)?;
                let cc_json: String = row.get(10)?;
                let bcc_json: String = row.get(11)?;

                Ok(Email {
                    id: row.get(0)?,
                    download_item_id: row.get(1)?,
                    credential_id: row.get(2)?,
                    uid: row.get::<_, i32>(3)? as u32,
                    folder_id: row.get(4)?,
                    message_id: row.get(5)?,
                    subject: row.get(6)?,
                    from_address: row.get(7)?,
                    from_name: row.get(8)?,
                    to_addresses: serde_json::from_str(&to_json).unwrap_or_default(),
                    cc_addresses: serde_json::from_str(&cc_json).unwrap_or_default(),
                    bcc_addresses: serde_json::from_str(&bcc_json).unwrap_or_default(),
                    reply_to: row.get(12)?,
                    date_sent: row.get(13)?,
                    date_received: row.get(14)?,
                    body_text: row.get(15)?,
                    body_html: row.get(16)?,
                    is_read: row.get(17)?,
                    is_flagged: row.get(18)?,
                    is_draft: row.get(19)?,
                    is_answered: row.get(20)?,
                    has_attachments: row.get(21)?,
                    attachment_count: row.get(22)?,
                    size_bytes: row.get(23)?,
                    thread_id: row.get(24)?,
                    created_at: row.get(25)?,
                    updated_at: row.get(26)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(emails)
    })
    .await?
}

/// List minimal email fields for scanning
pub async fn list_email_scan_rows(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
    max_emails: Option<usize>,
) -> Result<Vec<EmailScanRow>> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut query = String::from(
            "SELECT id, date_received, from_address, subject, body_text, body_html FROM emails",
        );
        let mut params: Vec<Value> = Vec::new();

        if let Some(cred) = credential_id {
            query.push_str(" WHERE credential_id = ?");
            params.push(Value::from(cred));
        }

        query.push_str(" ORDER BY date_received DESC");

        if let Some(limit) = max_emails {
            query.push_str(" LIMIT ?");
            params.push(Value::from(limit as i64));
        }

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            Ok(EmailScanRow {
                email_id: row.get(0)?,
                date_received: row.get(1)?,
                from_address: row.get(2)?,
                subject: row.get(3)?,
                body_text: row.get(4)?,
                body_html: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    })
    .await?
}

/// List minimal email fields for scanning using matched document IDs
pub async fn list_email_scan_rows_by_document_ids(
    conn: AsyncDbConnection,
    document_ids: &[i64],
    credential_id: Option<i64>,
    max_emails: Option<usize>,
) -> Result<Vec<EmailScanRow>> {
    if document_ids.is_empty() {
        return Ok(Vec::new());
    }

    let document_ids = document_ids.to_vec();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut seen_email_ids = HashSet::new();
        let mut rows: Vec<(i64, EmailScanRow)> = Vec::new();

        for chunk in document_ids.chunks(900) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let mut query = format!(
                "SELECT e.id, e.date_received, e.from_address, e.subject, e.body_text, e.body_html
                 FROM documents d
                 JOIN emails e ON e.id = d.email_id
                 WHERE d.id IN ({})
                   AND d.kind = 'email'",
                placeholders
            );
            let mut params: Vec<Value> = chunk.iter().copied().map(Value::from).collect();

            if let Some(cred) = credential_id {
                query.push_str(" AND e.credential_id = ?");
                params.push(Value::from(cred));
            }

            let mut stmt = conn.prepare(&query)?;
            let mapped = stmt.query_map(params_from_iter(params), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    EmailScanRow {
                        email_id: row.get(0)?,
                        date_received: row.get(1)?,
                        from_address: row.get(2)?,
                        subject: row.get(3)?,
                        body_text: row.get(4)?,
                        body_html: row.get(5)?,
                    },
                ))
            })?;

            for item in mapped {
                let (email_id, date_received, scan_row) = item?;
                if seen_email_ids.insert(email_id) {
                    rows.push((date_received, scan_row));
                }
            }
        }

        rows.sort_by(|a, b| b.0.cmp(&a.0));
        if let Some(limit) = max_emails {
            rows.truncate(limit);
        }

        Ok(rows.into_iter().map(|(_, row)| row).collect())
    })
    .await?
}

pub async fn list_template_candidate_emails_by_sender_and_document_ids(
    conn: AsyncDbConnection,
    sender_email: &str,
    document_ids: &[i64],
    credential_id: Option<i64>,
    max_emails: Option<usize>,
) -> Result<Vec<TemplateCandidateEmailRow>> {
    if document_ids.is_empty() {
        return Ok(Vec::new());
    }

    let sender_email = sender_email.to_string();
    let document_ids = document_ids.to_vec();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut seen_email_ids = HashSet::new();
        let mut rows: Vec<(i64, TemplateCandidateEmailRow)> = Vec::new();

        for chunk in document_ids.chunks(900) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let mut query = format!(
                "SELECT e.id, e.date_received, e.from_address, e.subject, e.body_text, e.body_html
                 FROM documents d
                 JOIN emails e ON e.id = d.email_id
                 WHERE d.id IN ({})
                   AND d.kind = 'email'
                   AND LOWER(e.from_address) = LOWER(?)",
                placeholders
            );
            let mut params: Vec<Value> = chunk.iter().copied().map(Value::from).collect();
            params.push(Value::from(sender_email.clone()));

            if let Some(cred) = credential_id {
                query.push_str(" AND e.credential_id = ?");
                params.push(Value::from(cred));
            }

            let mut stmt = conn.prepare(&query)?;
            let mapped = stmt.query_map(params_from_iter(params), |row| {
                Ok((
                    row.get::<_, i64>(1)?,
                    TemplateCandidateEmailRow {
                        email_id: row.get(0)?,
                        from_address: row.get(2)?,
                        subject: row.get(3)?,
                        body_text: row.get(4)?,
                        body_html: row.get(5)?,
                    },
                ))
            })?;

            for item in mapped {
                let (date_received, candidate_row) = item?;
                if seen_email_ids.insert(candidate_row.email_id) {
                    rows.push((date_received, candidate_row));
                }
            }
        }

        rows.sort_by(|a, b| b.0.cmp(&a.0));
        if let Some(limit) = max_emails {
            rows.truncate(limit);
        }
        Ok(rows.into_iter().map(|(_, row)| row).collect())
    })
    .await?
}

pub async fn list_unmatched_template_candidate_emails_by_sender_and_document_ids(
    conn: AsyncDbConnection,
    sender_email: &str,
    document_ids: &[i64],
    credential_id: Option<i64>,
    max_emails: Option<usize>,
) -> Result<Vec<TemplateCandidateEmailRow>> {
    if document_ids.is_empty() {
        return Ok(Vec::new());
    }

    let sender_email = sender_email.to_string();
    let document_ids = document_ids.to_vec();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut seen_email_ids = HashSet::new();
        let mut rows: Vec<(i64, TemplateCandidateEmailRow)> = Vec::new();

        for chunk in document_ids.chunks(900) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let mut query = format!(
                "SELECT e.id, e.date_received, e.from_address, e.subject, e.body_text, e.body_html
                 FROM documents d
                 JOIN emails e ON e.id = d.email_id
                 WHERE d.id IN ({})
                   AND d.kind = 'email'
                   AND LOWER(e.from_address) = LOWER(?)
                   AND NOT EXISTS (
                       SELECT 1
                       FROM financial_template_email_links l
                       JOIN financial_extraction_templates t ON t.id = l.template_id
                       WHERE l.email_id = e.id
                         AND t.data_source_type = 'email'
                         AND LOWER(t.data_source_id) = LOWER(e.from_address)
                         AND t.status = 'active'
                   )",
                placeholders
            );
            let mut params: Vec<Value> = chunk.iter().copied().map(Value::from).collect();
            params.push(Value::from(sender_email.clone()));

            if let Some(cred) = credential_id {
                query.push_str(" AND e.credential_id = ?");
                params.push(Value::from(cred));
            }

            let mut stmt = conn.prepare(&query)?;
            let mapped = stmt.query_map(params_from_iter(params), |row| {
                Ok((
                    row.get::<_, i64>(1)?,
                    TemplateCandidateEmailRow {
                        email_id: row.get(0)?,
                        from_address: row.get(2)?,
                        subject: row.get(3)?,
                        body_text: row.get(4)?,
                        body_html: row.get(5)?,
                    },
                ))
            })?;

            for item in mapped {
                let (date_received, candidate_row) = item?;
                if seen_email_ids.insert(candidate_row.email_id) {
                    rows.push((date_received, candidate_row));
                }
            }
        }

        rows.sort_by(|a, b| b.0.cmp(&a.0));
        if let Some(limit) = max_emails {
            rows.truncate(limit);
        }
        Ok(rows.into_iter().map(|(_, row)| row).collect())
    })
    .await?
}

/// Count total emails (optionally scoped by credential)
pub async fn count_emails(conn: AsyncDbConnection, credential_id: Option<i64>) -> Result<i64> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut query = String::from("SELECT COUNT(*) FROM emails");
        let mut params: Vec<Value> = Vec::new();

        if let Some(cred) = credential_id {
            query.push_str(" WHERE credential_id = ?");
            params.push(Value::from(cred));
        }

        let count: i64 = conn.query_row(&query, params_from_iter(params), |row| row.get(0))?;
        Ok(count)
    })
    .await?
}

/// List emails by label with pagination
pub async fn list_emails_by_label(
    conn: AsyncDbConnection,
    label_id: i64,
    limit: usize,
    offset: usize,
) -> Result<Vec<Email>> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let query = format!(
            "SELECT e.id, e.download_item_id, e.credential_id, e.uid, e.folder_id, e.message_id, e.subject, e.from_address, e.from_name,
                    e.to_addresses, e.cc_addresses, e.bcc_addresses, e.reply_to, e.date_sent, e.date_received,
                    e.body_text, e.body_html, e.is_read, e.is_flagged, e.is_draft, e.is_answered,
                    e.has_attachments, e.attachment_count, e.size_bytes, e.thread_id,
                    e.created_at, e.updated_at
             FROM emails e
             INNER JOIN email_label_associations ela ON e.id = ela.email_id
             WHERE ela.label_id = {}
             ORDER BY e.date_received DESC
             LIMIT {} OFFSET {}",
            label_id, limit, offset
        );

        let mut stmt = conn.prepare(&query)?;
        let emails = stmt
            .query_map([], |row| {
                let to_json: String = row.get(9)?;
                let cc_json: String = row.get(10)?;
                let bcc_json: String = row.get(11)?;

                Ok(Email {
                    id: row.get(0)?,
                    download_item_id: row.get(1)?,
                    credential_id: row.get(2)?,
                    uid: row.get::<_, i32>(3)? as u32,
                    folder_id: row.get(4)?,
                    message_id: row.get(5)?,
                    subject: row.get(6)?,
                    from_address: row.get(7)?,
                    from_name: row.get(8)?,
                    to_addresses: serde_json::from_str(&to_json).unwrap_or_default(),
                    cc_addresses: serde_json::from_str(&cc_json).unwrap_or_default(),
                    bcc_addresses: serde_json::from_str(&bcc_json).unwrap_or_default(),
                    reply_to: row.get(12)?,
                    date_sent: row.get(13)?,
                    date_received: row.get(14)?,
                    body_text: row.get(15)?,
                    body_html: row.get(16)?,
                    is_read: row.get(17)?,
                    is_flagged: row.get(18)?,
                    is_draft: row.get(19)?,
                    is_answered: row.get(20)?,
                    has_attachments: row.get(21)?,
                    attachment_count: row.get(22)?,
                    size_bytes: row.get(23)?,
                    thread_id: row.get(24)?,
                    created_at: row.get(25)?,
                    updated_at: row.get(26)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(emails)
    })
    .await?
}

/// Insert attachment

/// Insert attachment
pub async fn insert_attachment(
    conn: AsyncDbConnection,
    email_id: i64,
    filename: &str,
    content_type: Option<&str>,
    size_bytes: Option<i32>,
    content_id: Option<&str>,
    file_path: &str,
    checksum: Option<&str>,
    is_inline: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let attachment_id: i64 = conn.query_row(
        "INSERT INTO email_attachments
         (email_id, filename, content_type, size_bytes, content_id, file_path, checksum,
          is_inline, extraction_status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          RETURNING id",
        rusqlite::params![
            email_id,
            filename,
            content_type,
            size_bytes,
            content_id,
            file_path,
            checksum,
            is_inline,
            "pending",
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(attachment_id)
}

/// Get attachments for an email
pub async fn get_email_attachments(
    conn: AsyncDbConnection,
    email_id: i64,
) -> Result<Vec<EmailAttachment>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, email_id, filename, content_type, size_bytes, content_id,
                file_path, checksum, is_inline, extraction_status, extracted_text,
                created_at, updated_at
         FROM email_attachments WHERE email_id = ?",
    )?;

    let attachments = stmt
        .query_map([email_id], |row| {
            let extraction_status_str: String = row.get(9)?;
            let extraction_status = match extraction_status_str.as_str() {
                "pending" => AttachmentExtractionStatus::Pending,
                "completed" => AttachmentExtractionStatus::Completed,
                "failed" => AttachmentExtractionStatus::Failed,
                "skipped" => AttachmentExtractionStatus::Skipped,
                _ => AttachmentExtractionStatus::Pending,
            };

            Ok(EmailAttachment {
                id: row.get(0)?,
                email_id: row.get(1)?,
                filename: row.get(2)?,
                content_type: row.get(3)?,
                size_bytes: row.get(4)?,
                content_id: row.get(5)?,
                file_path: row.get(6)?,
                checksum: row.get(7)?,
                is_inline: row.get(8)?,
                extraction_status,
                extracted_text: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(attachments)
}

pub async fn list_pending_attachments(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<EmailAttachment>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, email_id, filename, content_type, size_bytes, content_id,
                file_path, checksum, is_inline, extraction_status, extracted_text,
                created_at, updated_at
         FROM email_attachments
         WHERE extraction_status = 'pending'
         ORDER BY created_at ASC
         LIMIT ?",
    )?;

    let attachments = stmt
        .query_map([limit], |row| {
            let extraction_status_str: String = row.get(9)?;
            let extraction_status = match extraction_status_str.as_str() {
                "pending" => AttachmentExtractionStatus::Pending,
                "completed" => AttachmentExtractionStatus::Completed,
                "failed" => AttachmentExtractionStatus::Failed,
                "skipped" => AttachmentExtractionStatus::Skipped,
                _ => AttachmentExtractionStatus::Pending,
            };

            Ok(EmailAttachment {
                id: row.get(0)?,
                email_id: row.get(1)?,
                filename: row.get(2)?,
                content_type: row.get(3)?,
                size_bytes: row.get(4)?,
                content_id: row.get(5)?,
                file_path: row.get(6)?,
                checksum: row.get(7)?,
                is_inline: row.get(8)?,
                extraction_status,
                extracted_text: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(attachments)
}

pub async fn update_attachment_extraction_status(
    conn: AsyncDbConnection,
    attachment_id: i64,
    status: &str,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE email_attachments SET extraction_status = ?, updated_at = ? WHERE id = ?",
        rusqlite::params![status, now, attachment_id],
    )?;

    Ok(())
}
