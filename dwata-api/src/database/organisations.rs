use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Organisation;

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
