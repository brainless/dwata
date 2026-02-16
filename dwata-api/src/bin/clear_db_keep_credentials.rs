use anyhow::Context;
use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    let db_path =
        dwata_api::helpers::database::get_db_path().context("Failed to determine database path")?;

    if !db_path.exists() {
        println!("Database not found at: {:?}", db_path);
        return Ok(());
    }

    let mut conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database at {:?}", db_path))?;

    conn.execute("PRAGMA foreign_keys = OFF", [])?;

    let table_names: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT name
             FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND name != 'credentials_metadata'",
        )?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        names
    };

    let has_sequence_table: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'sqlite_sequence'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;

    let tx = conn.transaction()?;
    for table in &table_names {
        let sql = format!("DELETE FROM {}", table);
        tx.execute(&sql, [])?;
    }

    if has_sequence_table {
        tx.execute(
            "DELETE FROM sqlite_sequence WHERE name != 'credentials_metadata'",
            [],
        )?;
    }

    tx.commit()?;

    println!(
        "Cleared {} tables (kept credentials_metadata) in {:?}",
        table_names.len(),
        db_path
    );

    Ok(())
}
