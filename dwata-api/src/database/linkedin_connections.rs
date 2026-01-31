use crate::database::AsyncDbConnection;
use anyhow::Result;

pub async fn insert_linkedin_connection(
    conn: AsyncDbConnection,
    extraction_job_id: i64,
    contact_id: i64,
    connected_on: Option<String>,
    connected_date: Option<i64>,
    connection_source: String,
    direction: Option<String>,
    invitation_message: Option<String>,
    invitation_sent_at: Option<String>,
    company_at_connection: Option<String>,
    position_at_connection: Option<String>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let result: Result<i64, _> = conn.query_row(
        "SELECT id FROM linkedin_connections WHERE contact_id = ? AND extraction_job_id = ?",
        duckdb::params![contact_id, extraction_job_id],
        |row| row.get(0),
    );

    if let Ok(id) = result {
        return Ok(id);
    }

    let id: i64 = conn.query_row(
        "INSERT INTO linkedin_connections
         (extraction_job_id, contact_id, connected_on, connected_date, connection_source,
          direction, invitation_message, invitation_sent_at, company_at_connection,
          position_at_connection, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        duckdb::params![
            extraction_job_id,
            contact_id,
            connected_on.as_ref(),
            connected_date,
            &connection_source,
            direction.as_ref(),
            invitation_message.as_ref(),
            invitation_sent_at.as_ref(),
            company_at_connection.as_ref(),
            position_at_connection.as_ref(),
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}
