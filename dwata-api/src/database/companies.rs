use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Company;

pub async fn insert_company(
    conn: AsyncDbConnection,
    extraction_job_id: Option<i64>,
    name: String,
    description: Option<String>,
    industry: Option<String>,
    location: Option<String>,
    website: Option<String>,
    linkedin_url: Option<String>,
    confidence: Option<f32>,
    requires_review: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let id: i64 = conn.query_row(
        "INSERT INTO companies
         (extraction_job_id, name, description, industry, location, website, linkedin_url,
          confidence, requires_review, created_at, updated_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          RETURNING id",
        rusqlite::params![
            extraction_job_id,
            &name,
            description.as_ref(),
            industry.as_ref(),
            location.as_ref(),
            website.as_ref(),
            linkedin_url.as_ref(),
            confidence,
            requires_review,
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn get_or_create_company(
    conn: AsyncDbConnection,
    extraction_job_id: Option<i64>,
    name: String,
    location: Option<String>,
) -> Result<i64> {
    {
        let locked_conn = conn.lock().await;
        let result: Result<i64, _> = locked_conn.query_row(
            "SELECT id FROM companies WHERE name = ? AND (location = ? OR location IS NULL AND ? IS NULL)",
            rusqlite::params![&name, location.as_ref(), location.as_ref()],
            |row| row.get(0),
        );

        if let Ok(id) = result {
            return Ok(id);
        }
    }

    insert_company(
        conn,
        extraction_job_id,
        name,
        None,
        None,
        location,
        None,
        None,
        None,
        false,
    )
    .await
}

pub async fn get_company(conn: AsyncDbConnection, id: i64) -> Result<Company> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, extraction_job_id, name, description, industry, location, website,
                linkedin_url, confidence, requires_review, is_confirmed, is_duplicate,
                merged_into_company_id, created_at, updated_at
         FROM companies
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Company {
            id: row.get(0)?,
            extraction_job_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            industry: row.get(4)?,
            location: row.get(5)?,
            website: row.get(6)?,
            linkedin_url: row.get(7)?,
            confidence: row.get(8)?,
            requires_review: row.get(9)?,
            is_confirmed: row.get(10)?,
            is_duplicate: row.get(11)?,
            merged_into_company_id: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get company: {}", e))
}

pub async fn list_companies(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Company>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare("SELECT id FROM companies ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt.query_map([limit], |row| row.get::<_, i64>(0))?
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
