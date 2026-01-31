use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Position;

pub async fn insert_position(
    conn: AsyncDbConnection,
    extraction_job_id: Option<i64>,
    contact_id: i64,
    company_id: i64,
    title: String,
    description: Option<String>,
    location: Option<String>,
    started_on: Option<String>,
    finished_on: Option<String>,
    started_date: Option<i64>,
    finished_date: Option<i64>,
    is_current: bool,
    confidence: Option<f32>,
    requires_review: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let id: i64 = conn.query_row(
        "INSERT INTO positions
         (extraction_job_id, contact_id, company_id, title, description, location,
          started_on, finished_on, started_date, finished_date, is_current,
          confidence, requires_review, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        duckdb::params![
            extraction_job_id,
            contact_id,
            company_id,
            &title,
            description.as_ref(),
            location.as_ref(),
            started_on.as_ref(),
            finished_on.as_ref(),
            started_date,
            finished_date,
            is_current,
            confidence,
            requires_review,
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn get_position(conn: AsyncDbConnection, id: i64) -> Result<Position> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, extraction_job_id, contact_id, company_id, title, description, location,
                started_on, finished_on, started_date, finished_date, is_current,
                confidence, requires_review, is_confirmed, created_at, updated_at
         FROM positions
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Position {
            id: row.get(0)?,
            extraction_job_id: row.get(1)?,
            contact_id: row.get(2)?,
            company_id: row.get(3)?,
            title: row.get(4)?,
            description: row.get(5)?,
            location: row.get(6)?,
            started_on: row.get(7)?,
            finished_on: row.get(8)?,
            started_date: row.get(9)?,
            finished_date: row.get(10)?,
            is_current: row.get(11)?,
            confidence: row.get(12)?,
            requires_review: row.get(13)?,
            is_confirmed: row.get(14)?,
            created_at: row.get(15)?,
            updated_at: row.get(16)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get position: {}", e))
}

pub async fn list_positions(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Position>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare("SELECT id FROM positions ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt.query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut positions = Vec::new();
    for id in ids {
        if let Ok(position) = get_position(conn.clone(), id).await {
            positions.push(position);
        }
    }

    Ok(positions)
}

pub async fn list_contact_positions(conn: AsyncDbConnection, contact_id: i64) -> Result<Vec<Position>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare(
        "SELECT id FROM positions WHERE contact_id = ? ORDER BY started_date DESC, created_at DESC",
    )?;

    let ids: Vec<i64> = stmt.query_map([contact_id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut positions = Vec::new();
    for id in ids {
        if let Ok(position) = get_position(conn.clone(), id).await {
            positions.push(position);
        }
    }

    Ok(positions)
}
