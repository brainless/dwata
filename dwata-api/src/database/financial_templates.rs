use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::{DataSourceType, FinancialTemplateStatus, FinancialTemplateType};
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
