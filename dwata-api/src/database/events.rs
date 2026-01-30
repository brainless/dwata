use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Event;

pub async fn insert_event_from_extraction(
    conn: AsyncDbConnection,
    extraction_job_id: i64,
    email_id: Option<i64>,
    name: String,
    description: Option<String>,
    event_date: i64,
    location: Option<String>,
    attendees: Vec<String>,
    confidence: f32,
    requires_review: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let attendees_json = serde_json::to_string(&attendees)?;

    let id: i64 = conn.query_row(
        "INSERT INTO events
         (extraction_job_id, email_id, name, description, event_date, location, attendees,
          confidence, requires_review, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        duckdb::params![
            extraction_job_id,
            email_id,
            &name,
            description.as_ref(),
            event_date,
            location.as_ref(),
            &attendees_json,
            confidence,
            requires_review,
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn get_event(conn: AsyncDbConnection, id: i64) -> Result<Event> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, extraction_job_id, email_id, name, description, event_date, location,
                attendees, confidence, requires_review, is_confirmed, project_id, task_id,
                created_at, updated_at
         FROM events
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        let attendees_json: String = row.get(7)?;
        let attendees: serde_json::Value =
            serde_json::from_str(&attendees_json).unwrap_or(serde_json::json!([]));

        Ok(Event {
            id: row.get(0)?,
            extraction_job_id: row.get(1)?,
            email_id: row.get(2)?,
            name: row.get(3)?,
            description: row.get(4)?,
            event_date: row.get(5)?,
            location: row.get(6)?,
            attendees,
            confidence: row.get(8)?,
            requires_review: row.get(9)?,
            is_confirmed: row.get(10)?,
            project_id: row.get(11)?,
            task_id: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get event: {}", e))
}

pub async fn list_events(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Event>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare("SELECT id FROM events ORDER BY event_date DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt.query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut events = Vec::new();
    for id in ids {
        if let Ok(event) = get_event(conn.clone(), id).await {
            events.push(event);
        }
    }

    Ok(events)
}
