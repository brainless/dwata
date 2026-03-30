use actix_web::{web, HttpResponse, Result};
use std::sync::Arc;

use crate::database::Database;

/// Tables to preserve (credentials and email infrastructure)
const TABLES_TO_KEEP: &[&str] = &[
    "credentials_metadata",
    "emails",
    "email_attachments",
    "email_folders",
    "email_labels",
    "email_label_associations",
    "refinery_schema_history",
];

/// Clear all extracted data tables while keeping credentials and emails intact
pub async fn clear_extracted_data(db: web::Data<Arc<Database>>) -> Result<HttpResponse> {
    // Get a connection from the pool
    let mut conn = db.async_connection.get_blocking();

    conn.execute("PRAGMA foreign_keys = OFF", []).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to disable foreign keys: {}", e))
    })?;

    // Get all tables except the ones we want to keep
    let table_names: Vec<String> = {
        let placeholders = TABLES_TO_KEEP
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            "SELECT name
             FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND name NOT IN ({})",
            placeholders
        );

        let mut stmt = conn.prepare(&query).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to prepare query: {}", e))
        })?;

        let params: Vec<&dyn rusqlite::ToSql> = TABLES_TO_KEEP
            .iter()
            .map(|s| &*s as &dyn rusqlite::ToSql)
            .collect();

        let names = stmt
            .query_map(params.as_slice(), |row| row.get::<_, String>(0))
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!("Failed to query tables: {}", e))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Failed to collect table names: {}",
                    e
                ))
            })?;
        names
    };

    let has_sequence_table: bool =
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'sqlite_sequence'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to check sqlite_sequence: {}",
                e
            ))
        })? > 0;

    if table_names.is_empty() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "No tables to clear (all tables are in the keep list)",
            "tables_cleared": 0
        })));
    }

    // Execute deletions in a transaction
    let tx = conn.transaction().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to start transaction: {}", e))
    })?;

    for table in &table_names {
        let sql = format!("DELETE FROM {}", table);
        tx.execute(&sql, []).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to clear table {}: {}",
                table, e
            ))
        })?;
    }

    // Reset sequences for cleared tables
    if has_sequence_table {
        let sequence_placeholders = TABLES_TO_KEEP
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            "DELETE FROM sqlite_sequence WHERE name NOT IN ({})",
            sequence_placeholders
        );

        let params: Vec<&dyn rusqlite::ToSql> = TABLES_TO_KEEP
            .iter()
            .map(|s| &*s as &dyn rusqlite::ToSql)
            .collect();

        tx.execute(&sql, params.as_slice()).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to reset sequences: {}", e))
        })?;
    }

    tx.commit().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": format!("Cleared {} tables (kept credentials and emails)", table_names.len()),
        "tables_cleared": table_names.len(),
        "cleared_tables": table_names
    })))
}
