use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::{Organisation, OrganisationWithCounts};

pub async fn insert_organisation(
    conn: AsyncDbConnection,
    name: String,
    description: Option<String>,
    industry: Option<String>,
    email: Option<String>,
    location_id: Option<i64>,
    website: Option<String>,
    linkedin_url: Option<String>,
    search_summary: Option<String>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let id: i64 = conn.query_row(
        "INSERT INTO organisations
         (name, description, industry, email, location_id, website, linkedin_url, search_summary, created_at, updated_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          RETURNING id",
        rusqlite::params![
            &name,
            description.as_ref(),
            industry.as_ref(),
            email.as_ref(),
            location_id,
            website.as_ref(),
            linkedin_url.as_ref(),
            search_summary.as_ref(),
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

/// Returns the existing organisation ID if an entry with this email already exists,
/// then falls back to name match, then creates a new row.
/// The second element of the tuple is `true` when a new row was created.
pub async fn get_or_create_organisation_by_email(
    conn: AsyncDbConnection,
    name: String,
    email: String,
) -> Result<(i64, bool)> {
    {
        let locked = conn.lock().await;

        // Try email match first
        let by_email: Result<i64, _> = locked.query_row(
            "SELECT id FROM organisations WHERE email = ?",
            [&email],
            |row| row.get(0),
        );
        if let Ok(id) = by_email {
            return Ok((id, false));
        }

        // Try name match
        let by_name: Result<i64, _> = locked.query_row(
            "SELECT id FROM organisations WHERE name = ?",
            [&name],
            |row| row.get(0),
        );
        if let Ok(id) = by_name {
            return Ok((id, false));
        }
    }

    let id =
        insert_organisation(conn, name, None, None, Some(email), None, None, None, None).await?;
    Ok((id, true))
}

pub async fn get_or_create_organisation(
    conn: AsyncDbConnection,
    name: String,
    location_id: Option<i64>,
) -> Result<i64> {
    {
        let locked_conn = conn.lock().await;
        let result: Result<i64, _> = locked_conn.query_row(
            "SELECT id FROM organisations WHERE name = ? AND (location_id = ? OR location_id IS NULL AND ? IS NULL)",
            rusqlite::params![&name, location_id, location_id],
            |row| row.get(0),
        );

        if let Ok(id) = result {
            return Ok(id);
        }
    }

    insert_organisation(conn, name, None, None, None, location_id, None, None, None).await
}

pub async fn get_organisation(conn: AsyncDbConnection, id: i64) -> Result<Organisation> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, name, description, industry, email, location_id, website, linkedin_url,
                search_summary, created_at, updated_at
         FROM organisations
         WHERE id = ?",
    )?;

    let org = stmt
        .query_row([id], |row| {
            Ok(Organisation {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                industry: row.get(3)?,
                email: row.get(4)?,
                roles: vec![], // populated by a separate query below
                location_id: row.get(5)?,
                website: row.get(6)?,
                linkedin_url: row.get(7)?,
                search_summary: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|e| anyhow::anyhow!("Failed to get organisation: {}", e))?;

    // Load roles from the junction table
    let mut role_stmt =
        conn.prepare("SELECT role FROM organisation_roles WHERE organisation_id = ?")?;

    let roles: Vec<shared_types::OrganisationRole> = role_stmt
        .query_map([id], |row| {
            let role_str: String = row.get(0)?;
            Ok(role_str)
        })?
        .filter_map(|r| r.ok())
        .filter_map(|s| serde_json::from_value(serde_json::Value::String(s)).ok())
        .collect();

    Ok(Organisation { roles, ..org })
}

/// List organisations with email send/receive counts joined from the emails table.
/// Only organisations with a non-null email address are included.
pub async fn list_organisations_with_counts(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<OrganisationWithCounts>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare(
        "SELECT
             o.id, o.name, o.description, o.industry, o.email,
             o.location_id, o.website, o.linkedin_url, o.search_summary,
             o.created_at, o.updated_at,
             (SELECT COUNT(*) FROM emails e WHERE e.from_address = o.email) AS received_count,
             (SELECT COUNT(*) FROM emails e WHERE e.to_addresses LIKE '%' || o.email || '%') AS in_to_count
         FROM organisations o
         WHERE o.email IS NOT NULL
         ORDER BY received_count DESC, o.created_at DESC
         LIMIT ?",
    )?;

    let rows = stmt
        .query_map([limit as i64], |row| {
            Ok(OrganisationWithCounts {
                organisation: Organisation {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    industry: row.get(3)?,
                    email: row.get(4)?,
                    roles: vec![], // loaded separately below
                    location_id: row.get(5)?,
                    website: row.get(6)?,
                    linkedin_url: row.get(7)?,
                    search_summary: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                },
                received_count: row.get(11)?,
                in_to_count: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    // Load roles for each organisation
    let mut result = Vec::with_capacity(rows.len());
    for mut row in rows {
        let org_id = row.organisation.id;
        // Reuse conn to load roles
        let locked = conn.lock().await;
        let mut role_stmt =
            locked.prepare("SELECT role FROM organisation_roles WHERE organisation_id = ?")?;
        let roles: Vec<shared_types::OrganisationRole> = role_stmt
            .query_map([org_id], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .filter_map(|s| serde_json::from_value(serde_json::Value::String(s)).ok())
            .collect();
        row.organisation.roles = roles;
        result.push(row);
    }

    Ok(result)
}

pub async fn list_organisations(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<Organisation>> {
    let conn_guard = conn.lock().await;

    let mut stmt =
        conn_guard.prepare("SELECT id FROM organisations ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt
        .query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut organisations = Vec::new();
    for id in ids {
        if let Ok(org) = get_organisation(conn.clone(), id).await {
            organisations.push(org);
        }
    }

    Ok(organisations)
}
