use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Person;

pub async fn insert_person(
    conn: AsyncDbConnection,
    email_id: Option<i64>,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    organisation_id: Option<i64>,
    search_summary: Option<String>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    if let Some(email_addr) = &email {
        let existing: Result<i64, _> = conn.query_row(
            "SELECT id FROM persons WHERE email = ? LIMIT 1",
            [email_addr],
            |row| row.get(0),
        );

        if existing.is_ok() {
            return Err(anyhow::anyhow!(
                "Person with email {} already exists",
                email_addr
            ));
        }
    }

    let id: i64 = conn.query_row(
        "INSERT INTO persons
         (email_id, name, email, phone, organisation_id, search_summary, created_at, updated_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?)
          RETURNING id",
        rusqlite::params![
            email_id,
            &name,
            email.as_ref(),
            phone.as_ref(),
            organisation_id,
            search_summary.as_ref(),
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn get_person(conn: AsyncDbConnection, id: i64) -> Result<Person> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, email_id, name, email, phone, organisation_id, search_summary, created_at, updated_at
         FROM persons
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Person {
            id: row.get(0)?,
            email_id: row.get(1)?,
            name: row.get(2)?,
            email: row.get(3)?,
            phone: row.get(4)?,
            organisation_id: row.get(5)?,
            search_summary: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get person: {}", e))
}

pub async fn list_persons(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Person>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare("SELECT id FROM persons ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt
        .query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut persons = Vec::new();
    for id in ids {
        if let Ok(person) = get_person(conn.clone(), id).await {
            persons.push(person);
        }
    }

    Ok(persons)
}
