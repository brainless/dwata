use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Event;

pub async fn insert_event(
    conn: AsyncDbConnection,
    email_id: Option<i64>,
    name: String,
    description: Option<String>,
    event_date_raw: Option<String>,
    event_date: Option<i64>,
    location_id: Option<i64>,
    attendees: Vec<String>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let attendees_json = serde_json::to_string(&attendees)?;

    let id: i64 = conn.query_row(
        "INSERT INTO events
         (email_id, name, description, event_date_raw, event_date, location_id, attendees, created_at, updated_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
          RETURNING id",
        rusqlite::params![
            email_id,
            &name,
            description.as_ref(),
            event_date_raw,
            event_date,
            location_id,
            &attendees_json,
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
        "SELECT id, email_id, name, description, event_date_raw, event_date, location_id, attendees,
                project_id, task_id, created_at, updated_at
         FROM events
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        let attendees_json: String = row.get(7)?;
        let attendees: serde_json::Value =
            serde_json::from_str(&attendees_json).unwrap_or(serde_json::json!([]));

        Ok(Event {
            id: row.get(0)?,
            email_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            event_date_raw: row.get(4)?,
            event_date: row.get(5)?,
            location_id: row.get(6)?,
            attendees,
            project_id: row.get(8)?,
            task_id: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get event: {}", e))
}

pub async fn list_events(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Event>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare("SELECT id FROM events ORDER BY event_date DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt
        .query_map([limit], |row| row.get::<_, i64>(0))?
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
