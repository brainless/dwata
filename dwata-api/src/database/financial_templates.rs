use crate::database::AsyncDbConnection;
use anyhow::Result;
use rusqlite::params_from_iter;
use rusqlite::types::Value;
use shared_types::{DataSourceType, FinancialTemplateStatus, FinancialTemplateType};
use std::collections::HashMap;
use tokio::task;

fn data_source_type_to_str(data_source_type: DataSourceType) -> &'static str {
    match data_source_type {
        DataSourceType::Email => "email",
        DataSourceType::Imap => "imap",
        DataSourceType::BankStatement => "bank-statement",
        DataSourceType::CreditCardStatement => "credit-card-statement",
        DataSourceType::BankFeed => "bank-feed",
        DataSourceType::CsvUpload => "csv-upload",
        DataSourceType::Manual => "manual",
        DataSourceType::Unknown => "unknown",
    }
}

fn template_type_to_str(template_type: FinancialTemplateType) -> &'static str {
    match template_type {
        FinancialTemplateType::Bill => "bill",
        FinancialTemplateType::Transaction => "transaction",
    }
}

fn template_status_to_str(status: FinancialTemplateStatus) -> &'static str {
    match status {
        FinancialTemplateStatus::Active => "active",
        FinancialTemplateStatus::Superseded => "superseded",
        FinancialTemplateStatus::Disabled => "disabled",
    }
}

pub async fn insert_template(
    conn: AsyncDbConnection,
    data_source_type: DataSourceType,
    data_source_id: &str,
    template_type: FinancialTemplateType,
    template_body: &str,
) -> Result<i64> {
    let data_source_id = data_source_id.to_string();
    let template_body = template_body.to_string();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let now = chrono::Utc::now().timestamp_millis();
        let id = conn.query_row(
            "INSERT INTO financial_extraction_templates
             (data_source_type, data_source_id, template_type, template_body, status, version, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 1, ?, ?)
             RETURNING id",
            rusqlite::params![
                data_source_type_to_str(data_source_type),
                data_source_id,
                template_type_to_str(template_type),
                template_body,
                template_status_to_str(FinancialTemplateStatus::Active),
                now,
                now
            ],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(id)
    })
    .await?
}

pub async fn insert_template_variable(
    conn: AsyncDbConnection,
    template_id: i64,
    placeholder_name: &str,
    target_field: &str,
) -> Result<i64> {
    let placeholder_name = placeholder_name.to_string();
    let target_field = target_field.to_string();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let now = chrono::Utc::now().timestamp_millis();
        let id = conn.query_row(
            "INSERT INTO financial_template_variables
             (template_id, placeholder_name, target_field, created_at)
             VALUES (?, ?, ?, ?)
             RETURNING id",
            rusqlite::params![template_id, placeholder_name, target_field, now],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(id)
    })
    .await?
}

pub async fn insert_template_applicability(
    conn: AsyncDbConnection,
    template_id: i64,
    data_source_type: DataSourceType,
    data_source_id: &str,
    match_score: Option<f64>,
) -> Result<i64> {
    let data_source_id = data_source_id.to_string();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let now = chrono::Utc::now().timestamp_millis();
        let id = conn.query_row(
            "INSERT INTO financial_template_applicability
             (template_id, data_source_type, data_source_id, match_score, created_at)
             VALUES (?, ?, ?, ?, ?)
             RETURNING id",
            rusqlite::params![
                template_id,
                data_source_type_to_str(data_source_type),
                data_source_id,
                match_score,
                now
            ],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(id)
    })
    .await?
}

pub async fn insert_template_email_link(
    conn: AsyncDbConnection,
    template_id: i64,
    email_id: i64,
    match_score: Option<f64>,
) -> Result<i64> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let now = chrono::Utc::now().timestamp_millis();
        let id = conn.query_row(
            "INSERT INTO financial_template_email_links
             (template_id, email_id, match_score, created_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(template_id, email_id) DO UPDATE SET match_score=excluded.match_score
             RETURNING id",
            rusqlite::params![template_id, email_id, match_score, now],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(id)
    })
    .await?
}

#[derive(Debug, Clone)]
pub struct FinancialTemplateVariableRow {
    pub placeholder_name: String,
    pub target_field: String,
}

#[derive(Debug, Clone)]
pub struct SenderFinancialTemplateRow {
    pub template_id: i64,
    pub template_type: FinancialTemplateType,
    pub template_body: String,
    pub variables: Vec<FinancialTemplateVariableRow>,
}

#[derive(Debug, Clone)]
pub struct StoredFinancialTemplateWithVariablesRow {
    pub id: i64,
    pub data_source_type: DataSourceType,
    pub data_source_id: String,
    pub template_type: FinancialTemplateType,
    pub template_body: String,
    pub status: FinancialTemplateStatus,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub variables: Vec<FinancialTemplateVariableRow>,
}

fn data_source_type_from_str(value: &str) -> Result<DataSourceType> {
    match value {
        "email" => Ok(DataSourceType::Email),
        "imap" => Ok(DataSourceType::Imap),
        "bank-statement" => Ok(DataSourceType::BankStatement),
        "credit-card-statement" => Ok(DataSourceType::CreditCardStatement),
        "bank-feed" => Ok(DataSourceType::BankFeed),
        "csv-upload" => Ok(DataSourceType::CsvUpload),
        "manual" => Ok(DataSourceType::Manual),
        "unknown" => Ok(DataSourceType::Unknown),
        other => Err(anyhow::anyhow!("unknown data source type in DB: {other}")),
    }
}

fn template_type_from_str(value: &str) -> Result<FinancialTemplateType> {
    match value {
        "bill" => Ok(FinancialTemplateType::Bill),
        "transaction" => Ok(FinancialTemplateType::Transaction),
        other => Err(anyhow::anyhow!(
            "unknown financial template type in DB: {other}"
        )),
    }
}

fn template_status_from_str(value: &str) -> Result<FinancialTemplateStatus> {
    match value {
        "active" => Ok(FinancialTemplateStatus::Active),
        "superseded" => Ok(FinancialTemplateStatus::Superseded),
        "disabled" => Ok(FinancialTemplateStatus::Disabled),
        other => Err(anyhow::anyhow!(
            "unknown financial template status in DB: {other}"
        )),
    }
}

pub async fn list_templates_with_variables_by_sender(
    conn: AsyncDbConnection,
    sender_email: &str,
) -> Result<Vec<SenderFinancialTemplateRow>> {
    let sender_email = sender_email.to_string();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.template_type, t.template_body, v.placeholder_name, v.target_field
             FROM financial_extraction_templates t
             LEFT JOIN financial_template_variables v ON v.template_id = t.id
             WHERE t.data_source_type = 'email'
               AND LOWER(t.data_source_id) = LOWER(?)
               AND t.status = 'active'
             ORDER BY t.id ASC, v.id ASC",
        )?;

        let mut rows = stmt.query([sender_email])?;
        let mut grouped: HashMap<i64, SenderFinancialTemplateRow> = HashMap::new();
        while let Some(row) = rows.next()? {
            let template_id: i64 = row.get(0)?;
            let template_type_str: String = row.get(1)?;
            let template_body: String = row.get(2)?;
            let template_type = template_type_from_str(&template_type_str)?;

            let entry = grouped
                .entry(template_id)
                .or_insert_with(|| SenderFinancialTemplateRow {
                    template_id,
                    template_type,
                    template_body,
                    variables: Vec::new(),
                });

            let placeholder_name: Option<String> = row.get(3)?;
            let target_field: Option<String> = row.get(4)?;
            if let (Some(placeholder_name), Some(target_field)) = (placeholder_name, target_field) {
                entry.variables.push(FinancialTemplateVariableRow {
                    placeholder_name,
                    target_field,
                });
            }
        }

        let mut templates: Vec<SenderFinancialTemplateRow> = grouped.into_values().collect();
        templates.sort_by_key(|t| t.template_id);
        Ok(templates)
    })
    .await?
}

pub async fn list_active_templates_with_variables(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<StoredFinancialTemplateWithVariablesRow>> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.data_source_type, t.data_source_id, t.template_type, t.template_body, t.status, t.version, t.created_at, t.updated_at, v.placeholder_name, v.target_field
             FROM financial_extraction_templates t
             LEFT JOIN financial_template_variables v ON v.template_id = t.id
             WHERE t.status = 'active'
             ORDER BY t.updated_at DESC, t.id DESC, v.id ASC
             LIMIT ?",
        )?;

        let mut rows = stmt.query([limit as i64])?;
        let mut grouped: HashMap<i64, StoredFinancialTemplateWithVariablesRow> = HashMap::new();
        while let Some(row) = rows.next()? {
            let template_id: i64 = row.get(0)?;
            let data_source_type = data_source_type_from_str(&row.get::<_, String>(1)?)?;
            let template_type = template_type_from_str(&row.get::<_, String>(3)?)?;
            let status = template_status_from_str(&row.get::<_, String>(5)?)?;

            let entry = grouped.entry(template_id).or_insert_with(|| StoredFinancialTemplateWithVariablesRow {
                id: template_id,
                data_source_type,
                data_source_id: row.get(2).unwrap_or_default(),
                template_type,
                template_body: row.get(4).unwrap_or_default(),
                status,
                version: row.get(6).unwrap_or(1),
                created_at: row.get(7).unwrap_or_default(),
                updated_at: row.get(8).unwrap_or_default(),
                variables: Vec::new(),
            });

            let placeholder_name: Option<String> = row.get(9)?;
            let target_field: Option<String> = row.get(10)?;
            if let (Some(placeholder_name), Some(target_field)) = (placeholder_name, target_field) {
                entry.variables.push(FinancialTemplateVariableRow {
                    placeholder_name,
                    target_field,
                });
            }
        }

        let mut templates: Vec<StoredFinancialTemplateWithVariablesRow> = grouped.into_values().collect();
        templates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then_with(|| b.id.cmp(&a.id)));
        Ok(templates)
    })
    .await?
}

pub async fn delete_templates_by_ids(
    conn: AsyncDbConnection,
    template_ids: &[i64],
) -> Result<usize> {
    if template_ids.is_empty() {
        return Ok(0);
    }

    let template_ids = template_ids.to_vec();
    task::spawn_blocking(move || {
        let mut conn = conn.get_blocking();
        let tx = conn.transaction()?;
        let mut deleted_count = 0usize;

        for chunk in template_ids.chunks(900) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "DELETE FROM financial_extraction_templates
                 WHERE id IN ({})",
                placeholders
            );
            let params: Vec<Value> = chunk.iter().copied().map(Value::from).collect();
            deleted_count += tx.execute(&query, params_from_iter(params))?;
        }

        tx.commit()?;
        Ok(deleted_count)
    })
    .await?
}

pub async fn list_anchor_email_ids_by_template_ids(
    conn: AsyncDbConnection,
    template_ids: &[i64],
    credential_id: Option<i64>,
) -> Result<HashMap<i64, i64>> {
    if template_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let template_ids = template_ids.to_vec();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut out: HashMap<i64, i64> = HashMap::new();

        for chunk in template_ids.chunks(900) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let mut query = format!(
                "SELECT l.template_id, MIN(l.email_id) AS anchor_email_id
                 FROM financial_template_email_links l
                 JOIN emails e ON e.id = l.email_id
                 WHERE l.template_id IN ({})",
                placeholders
            );
            let mut params: Vec<Value> = chunk.iter().copied().map(Value::from).collect();
            if let Some(cred) = credential_id {
                query.push_str(" AND e.credential_id = ?");
                params.push(Value::from(cred));
            }
            query.push_str(" GROUP BY l.template_id");

            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map(params_from_iter(params), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (template_id, email_id) = row?;
                out.insert(template_id, email_id);
            }
        }

        Ok(out)
    })
    .await?
}

pub async fn list_sender_max_cluster_sizes(
    conn: AsyncDbConnection,
    sender_emails: &[String],
    credential_id: Option<i64>,
) -> Result<HashMap<String, usize>> {
    if sender_emails.is_empty() {
        return Ok(HashMap::new());
    }

    let sender_emails = sender_emails.to_vec();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut out: HashMap<String, usize> = HashMap::new();

        for chunk in sender_emails.chunks(900) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let credential_filter = if credential_id.is_some() {
                "AND e.credential_id = ?"
            } else {
                ""
            };
            let query = format!(
                "SELECT t.data_source_id,
                        MAX(
                            (
                                SELECT COUNT(*)
                                FROM financial_template_email_links l
                                JOIN emails e ON e.id = l.email_id
                                WHERE l.template_id = t.id
                                  {credential_filter}
                            )
                        ) AS max_cluster_size
                 FROM financial_extraction_templates t
                 WHERE t.data_source_type = 'email'
                   AND t.status = 'active'
                   AND LOWER(t.data_source_id) IN ({placeholders})
                 GROUP BY t.data_source_id"
            );

            let mut params: Vec<Value> = Vec::new();
            if let Some(cred) = credential_id {
                params.push(Value::from(cred));
            }
            params.extend(
                chunk
                    .iter()
                    .map(|s| s.trim().to_ascii_lowercase())
                    .map(Value::from),
            );

            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map(params_from_iter(params), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (sender, max_cluster_size) = row?;
                out.insert(
                    sender.to_ascii_lowercase(),
                    max_cluster_size.max(0) as usize,
                );
            }
        }

        Ok(out)
    })
    .await?
}
