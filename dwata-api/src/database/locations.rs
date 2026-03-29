use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::Location;

pub async fn get_location(conn: AsyncDbConnection, id: i64) -> Result<Location> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, name, address_line1, address_line2, city, region, country_code,
                postal_code, search_summary, created_at, updated_at
         FROM locations
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Location {
            id: row.get(0)?,
            name: row.get(1)?,
            address_line1: row.get(2)?,
            address_line2: row.get(3)?,
            city: row.get(4)?,
            region: row.get(5)?,
            country_code: row.get(6)?,
            postal_code: row.get(7)?,
            search_summary: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get location: {}", e))
}

pub async fn list_locations(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Location>> {
    let conn_guard = conn.lock().await;

    let mut stmt =
        conn_guard.prepare("SELECT id FROM locations ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt
        .query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut locations = Vec::new();
    for id in ids {
        if let Ok(location) = get_location(conn.clone(), id).await {
            locations.push(location);
        }
    }

    Ok(locations)
}
