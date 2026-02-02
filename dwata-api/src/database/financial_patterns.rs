use crate::database::AsyncDbConnection;
use shared_types::FinancialPattern;
use duckdb::{params, Row};
use anyhow::Result;

pub async fn list_patterns(
    db_conn: AsyncDbConnection,
    active_only: bool,
    is_default: Option<bool>,
    document_type: Option<String>,
) -> Result<Vec<FinancialPattern>> {
    let conn = db_conn.lock().await;

    let mut patterns = Vec::new();

    if active_only || is_default.is_some() || document_type.is_some() {
        let mut query = "SELECT * FROM financial_patterns WHERE 1=1".to_string();

        if active_only {
            query.push_str(" AND is_active = true");
        }

        if let Some(default_val) = is_default {
            query.push_str(&format!(" AND is_default = {}", if default_val { "true" } else { "false" }));
        }

        if let Some(doc_type) = document_type {
            query.push_str(&format!(" AND document_type = '{}'", doc_type));
        }

        query.push_str(" ORDER BY id");

        let mut stmt = conn.prepare(&query)?;

        let rows = stmt.query_map([], |row| map_row_to_pattern(row))?;

        for row in rows {
            patterns.push(row?);
        }
    } else {
        let mut stmt = conn.prepare("SELECT * FROM financial_patterns ORDER BY id")?;

        let rows = stmt.query_map([], |row| map_row_to_pattern(row))?;

        for row in rows {
            patterns.push(row?);
        }
    }

    Ok(patterns)
}

pub async fn get_pattern(
    db_conn: AsyncDbConnection,
    id: i64,
) -> Result<FinancialPattern> {
    let conn = db_conn.lock().await;

    let mut stmt = conn.prepare("SELECT * FROM financial_patterns WHERE id = ?1")?;

    let pattern = stmt.query_row(params![id], |row| map_row_to_pattern(row))?;

    Ok(pattern)
}

pub async fn list_active_patterns(
    db_conn: AsyncDbConnection,
) -> Result<Vec<FinancialPattern>> {
    list_patterns(db_conn, true, None, None).await
}

pub async fn insert_pattern(
    db_conn: AsyncDbConnection,
    pattern: &FinancialPattern,
) -> Result<i64> {
    let conn = db_conn.lock().await;

    let id: i64 = conn.query_row(
        "INSERT INTO financial_patterns (name, regex_pattern, description, document_type, status, confidence,
         amount_group, vendor_group, date_group, is_default, is_active, match_count, last_matched_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         RETURNING id",
        params![
            pattern.name,
            pattern.regex_pattern,
            pattern.description.as_deref(),
            &pattern.document_type,
            &pattern.status,
            pattern.confidence,
            pattern.amount_group,
            pattern.vendor_group,
            pattern.date_group,
            pattern.is_default,
            pattern.is_active,
            pattern.match_count,
            pattern.last_matched_at,
            pattern.created_at,
            pattern.updated_at,
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn update_pattern(
    db_conn: AsyncDbConnection,
    id: i64,
    pattern: &FinancialPattern,
) -> Result<()> {
    let conn = db_conn.lock().await;

    conn.execute(
        "UPDATE financial_patterns
         SET name = ?1, regex_pattern = ?2, description = ?3, document_type = ?4, status = ?5,
             confidence = ?6, amount_group = ?7, vendor_group = ?8, date_group = ?9,
             is_active = ?10, updated_at = ?11
         WHERE id = ?12",
        params![
            pattern.name,
            pattern.regex_pattern,
            pattern.description.as_deref(),
            &pattern.document_type,
            &pattern.status,
            pattern.confidence,
            pattern.amount_group,
            pattern.vendor_group,
            pattern.date_group,
            pattern.is_active,
            pattern.updated_at,
            id,
        ],
    )?;

    Ok(())
}

pub async fn toggle_pattern_active(
    db_conn: AsyncDbConnection,
    id: i64,
    is_active: bool,
) -> Result<()> {
    let conn = db_conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "UPDATE financial_patterns SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
        params![is_active, now, id],
    )?;

    Ok(())
}

pub async fn pattern_name_exists(
    db_conn: AsyncDbConnection,
    name: &str,
    exclude_id: Option<i64>,
) -> Result<bool> {
    let conn = db_conn.lock().await;

    let count: i64 = if let Some(exclude_id) = exclude_id {
        conn.query_row(
            "SELECT COUNT(*) FROM financial_patterns WHERE name = ?1 AND id != ?2",
            params![name, exclude_id],
            |row| row.get(0)
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM financial_patterns WHERE name = ?1",
            params![name],
            |row| row.get(0)
        )?
    };

    Ok(count > 0)
}

pub async fn pattern_regex_exists(
    db_conn: AsyncDbConnection,
    regex_pattern: &str,
    exclude_id: Option<i64>,
) -> Result<bool> {
    let conn = db_conn.lock().await;

    let count: i64 = if let Some(exclude_id) = exclude_id {
        conn.query_row(
            "SELECT COUNT(*) FROM financial_patterns WHERE regex_pattern = ?1 AND id != ?2",
            params![regex_pattern, exclude_id],
            |row| row.get(0)
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM financial_patterns WHERE regex_pattern = ?1",
            params![regex_pattern],
            |row| row.get(0)
        )?
    };

    Ok(count > 0)
}

pub async fn increment_match_count(
    db_conn: AsyncDbConnection,
    id: i64,
) -> Result<()> {
    let conn = db_conn.lock().await;

    conn.execute(
        "UPDATE financial_patterns SET match_count = match_count + 1 WHERE id = ?1",
        params![id],
    )?;

    Ok(())
}

pub async fn update_last_matched(
    db_conn: AsyncDbConnection,
    id: i64,
    timestamp: i64,
) -> Result<()> {
    let conn = db_conn.lock().await;

    conn.execute(
        "UPDATE financial_patterns SET last_matched_at = ?1 WHERE id = ?2",
        params![timestamp, id],
    )?;

    Ok(())
}

fn map_row_to_pattern(row: &Row) -> duckdb::Result<FinancialPattern> {
    Ok(FinancialPattern {
        id: row.get(0)?,
        name: row.get(1)?,
        regex_pattern: row.get(2)?,
        description: row.get(3)?,
        document_type: row.get(4)?,
        status: row.get(5)?,
        confidence: row.get(6)?,
        amount_group: row.get(7)?,
        vendor_group: row.get(8)?,
        date_group: row.get(9)?,
        is_default: row.get(10)?,
        is_active: row.get(11)?,
        match_count: row.get(12)?,
        last_matched_at: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}
