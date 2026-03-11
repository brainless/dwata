use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Company;

pub async fn insert_company(
    conn: AsyncDbConnection,
    name: String,
    description: Option<String>,
    industry: Option<String>,
    location_id: Option<i64>,
    website: Option<String>,
    linkedin_url: Option<String>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let id: i64 = conn.query_row(
        "INSERT INTO companies
         (name, description, industry, location_id, website, linkedin_url, created_at, updated_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?)
          RETURNING id",
        rusqlite::params![
            &name,
            description.as_ref(),
            industry.as_ref(),
            location_id,
            website.as_ref(),
            linkedin_url.as_ref(),
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn get_or_create_company(
    conn: AsyncDbConnection,
    name: String,
    location_id: Option<i64>,
) -> Result<i64> {
    {
        let locked_conn = conn.lock().await;
        let result: Result<i64, _> = locked_conn.query_row(
            "SELECT id FROM companies WHERE name = ? AND (location_id = ? OR location_id IS NULL AND ? IS NULL)",
            rusqlite::params![&name, location_id, location_id],
            |row| row.get(0),
        );

        if let Ok(id) = result {
            return Ok(id);
        }
    }

    insert_company(conn, name, None, None, location_id, None, None).await
}

pub async fn get_company(conn: AsyncDbConnection, id: i64) -> Result<Company> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, name, description, industry, location_id, website, linkedin_url,
                created_at, updated_at
         FROM companies
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Company {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            industry: row.get(3)?,
            location_id: row.get(4)?,
            website: row.get(5)?,
            linkedin_url: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get company: {}", e))
}

pub async fn list_companies(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Company>> {
    let conn_guard = conn.lock().await;

    let mut stmt =
        conn_guard.prepare("SELECT id FROM companies ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt
        .query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut companies = Vec::new();
    for id in ids {
        if let Ok(company) = get_company(conn.clone(), id).await {
            companies.push(company);
        }
    }

    Ok(companies)
}
