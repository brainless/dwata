use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Contact;

pub async fn insert_contact_from_extraction(
    conn: AsyncDbConnection,
    extraction_job_id: i64,
    email_id: Option<i64>,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    organization: Option<String>,
    confidence: f32,
    requires_review: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    if let Some(email_addr) = &email {
        let existing: Result<i64, _> = conn.query_row(
            "SELECT id FROM contacts WHERE email = ? LIMIT 1",
            [email_addr],
            |row| row.get(0),
        );

        if existing.is_ok() {
            return Err(anyhow::anyhow!("Contact with email {} already exists", email_addr));
        }
    }

    let id: i64 = conn.query_row(
        "INSERT INTO contacts
         (extraction_job_id, email_id, name, email, phone, organization,
          confidence, requires_review, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        duckdb::params![
            extraction_job_id,
            email_id,
            &name,
            email.as_ref(),
            phone.as_ref(),
            organization.as_ref(),
            confidence,
            requires_review,
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn get_contact(conn: AsyncDbConnection, id: i64) -> Result<Contact> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, extraction_job_id, email_id, name, email, phone, organization,
                confidence, requires_review, is_confirmed, is_duplicate, merged_into_contact_id,
                created_at, updated_at
         FROM contacts
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Contact {
            id: row.get(0)?,
            extraction_job_id: row.get(1)?,
            email_id: row.get(2)?,
            name: row.get(3)?,
            email: row.get(4)?,
            phone: row.get(5)?,
            organization: row.get(6)?,
            confidence: row.get(7)?,
            requires_review: row.get(8)?,
            is_confirmed: row.get(9)?,
            is_duplicate: row.get(10)?,
            merged_into_contact_id: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get contact: {}", e))
}

pub async fn list_contacts(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Contact>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare("SELECT id FROM contacts ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt.query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut contacts = Vec::new();
    for id in ids {
        if let Ok(contact) = get_contact(conn.clone(), id).await {
            contacts.push(contact);
        }
    }

    Ok(contacts)
}
