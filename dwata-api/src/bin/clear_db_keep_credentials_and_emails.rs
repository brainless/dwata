use anyhow::Context;
use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    let db_path = dwata_api::helpers::database::get_db_path()
        .context("Failed to determine database path")?;

    if !db_path.exists() {
        println!("Database not found at: {:?}", db_path);
        return Ok(());
    }

    let mut conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database at {:?}", db_path))?;

    conn.execute("PRAGMA foreign_keys = OFF", [])?;

    // Tables to keep: credentials_metadata, emails, email_attachments, email_folders,
    // email_labels, email_label_associations, emails_fts (FTS virtual table)
    let tables_to_keep = vec![
        "credentials_metadata",
        "emails",
        "email_attachments",
        "email_folders",
        "email_labels",
        "email_label_associations",
        "emails_fts",
    ];

    // Get all tables except the ones we want to keep
    // IMPORTANT: Also exclude FTS shadow tables (e.g., emails_fts_data, emails_fts_idx, etc.)
    let table_names: Vec<String> = {
        let placeholders = tables_to_keep
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            "SELECT name
             FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND name NOT LIKE 'emails_fts_%'
               AND name NOT IN ({})",
            placeholders
        );

        let mut stmt = conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> = tables_to_keep
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let names = stmt
            .query_map(params.as_slice(), |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        names
    };

    let has_sequence_table: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'sqlite_sequence'",
            [],
            |row| row.get::<_, i64>(0),
        )?
        > 0;

    if table_names.is_empty() {
        println!("No tables to clear (all tables are in the keep list)");
        return Ok(());
    }

    println!("Tables to be cleared:");
    for table in &table_names {
        println!("  - {}", table);
    }
    println!();

    let tx = conn.transaction()?;

    for table in &table_names {
        let sql = format!("DELETE FROM {}", table);
        tx.execute(&sql, [])?;
    }

    if has_sequence_table {
        // Reset sequences for all tables except the ones we're keeping
        let sequence_placeholders = tables_to_keep
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            "DELETE FROM sqlite_sequence WHERE name NOT IN ({})",
            sequence_placeholders
        );

        let params: Vec<&dyn rusqlite::ToSql> = tables_to_keep
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        tx.execute(&sql, params.as_slice())?;
    }

    tx.commit()?;

    // Rebuild the FTS index to ensure it's properly synchronized
    println!("Rebuilding FTS index...");
    conn.execute("INSERT INTO emails_fts(emails_fts) VALUES('rebuild')", [])?;

    println!(
        "✓ Cleared {} tables (kept credentials + emails) in {:?}",
        table_names.len(),
        db_path
    );

    Ok(())
}
