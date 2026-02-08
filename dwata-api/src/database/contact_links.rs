use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::{ContactLink, ContactLinkType};

pub async fn insert_contact_link(
    conn: AsyncDbConnection,
    contact_id: i64,
    link_type: ContactLinkType,
    url: String,
    label: Option<String>,
    is_primary: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();
    let link_type_str = format!("{:?}", link_type).to_lowercase();

    let result: Result<i64, _> = conn.query_row(
        "SELECT id FROM contact_links WHERE contact_id = ? AND link_type = ? AND url = ?",
        rusqlite::params![contact_id, &link_type_str, &url],
        |row| row.get(0),
    );

    if let Ok(id) = result {
        return Ok(id);
    }

    let id: i64 = conn.query_row(
        "INSERT INTO contact_links
         (contact_id, link_type, url, label, is_primary, created_at, updated_at)
          VALUES (?, ?, ?, ?, ?, ?, ?)
          RETURNING id",
        rusqlite::params![
            contact_id,
            &link_type_str,
            &url,
            label.as_ref(),
            is_primary,
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn get_contact_links(conn: AsyncDbConnection, contact_id: i64) -> Result<Vec<ContactLink>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, contact_id, link_type, url, label, is_primary, is_verified, created_at, updated_at
         FROM contact_links
         WHERE contact_id = ?",
    )?;

    let links = stmt.query_map([contact_id], |row| {
        let link_type_str: String = row.get(2)?;
        let link_type = match link_type_str.as_str() {
            "linkedin" => ContactLinkType::Linkedin,
            "github" => ContactLinkType::Github,
            "twitter" => ContactLinkType::Twitter,
            "personal" => ContactLinkType::Personal,
            _ => ContactLinkType::Other,
        };

        Ok(ContactLink {
            id: row.get(0)?,
            contact_id: row.get(1)?,
            link_type,
            url: row.get(3)?,
            label: row.get(4)?,
            is_primary: row.get(5)?,
            is_verified: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| anyhow::anyhow!("Failed to get contact links: {}", e))?;

    Ok(links)
}
