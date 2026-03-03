use crate::database::{financial_bills as bill_db, financial_transactions as txn_db, Database};
use anyhow::Result;
use dwata_agents::{
    extract_values_from_email, is_valid_bill_value, is_valid_txn_value, parse_amount, parse_date,
    TemplateEmailContent,
};
use serde::Serialize;
use shared_types::{
    Bill, BillStatus, DataSourceType, FinancialDocumentType, FinancialTransaction,
    TransactionCategory, TransactionParty, TransactionStatus,
};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct FinancialExtractionRunResponse {
    pub templates_scanned: usize,
    pub emails_scanned: usize,
    pub inserted_bills: usize,
    pub inserted_transactions: usize,
    pub discarded_rows: usize,
}

#[derive(Debug, Clone)]
enum TemplateKind {
    Bill,
    Transaction,
}

#[derive(Debug, Clone)]
struct TemplateRuntime {
    id: i64,
    template_body: String,
    kind: TemplateKind,
    placeholder_to_field: HashMap<String, String>,
}

pub async fn extract_financial_from_templates(
    db: Arc<Database>,
    credential_id: Option<i64>,
) -> Result<FinancialExtractionRunResponse> {
    let mut templates = load_templates(&db, credential_id).await?;
    let mut emails_scanned = 0usize;
    let mut inserted_bills = 0usize;
    let mut inserted_transactions = 0usize;
    let mut discarded_rows = 0usize;

    for template in templates.drain(..) {
        if template.placeholder_to_field.is_empty() {
            continue;
        }

        let email_rows = load_linked_emails_for_template(&db, template.id, credential_id).await?;
        for email in email_rows {
            emails_scanned += 1;
            let body = preferred_body_text(
                email.try_get::<Option<String>, _>("body_text")?.as_deref(),
                email.try_get::<Option<String>, _>("body_html")?.as_deref(),
            );
            let extracted = extract_values_from_email(
                &template.template_body,
                &TemplateEmailContent {
                    subject: email
                        .try_get::<Option<String>, _>("subject")?
                        .unwrap_or_default(),
                    body,
                },
            );
            if extracted.is_empty() {
                discarded_rows += 1;
                continue;
            }

            let mut field_values: HashMap<String, String> = HashMap::new();
            let mut invalid = false;
            for (placeholder, value) in extracted {
                if let Some(field) = template.placeholder_to_field.get(&placeholder) {
                    let ok = match template.kind {
                        TemplateKind::Bill => is_valid_bill_value(field, &value),
                        TemplateKind::Transaction => is_valid_txn_value(field, &value),
                    };
                    if ok {
                        field_values.insert(field.clone(), value);
                    } else {
                        invalid = true;
                        break;
                    }
                }
            }
            if invalid {
                discarded_rows += 1;
                continue;
            }

            let email_id: i64 = email.try_get("id")?;
            let received_ts: i64 = email.try_get("date_received")?;
            match template.kind {
                TemplateKind::Bill => {
                    if let Some(bill) = build_bill_from_fields(&field_values, email_id, received_ts)
                    {
                        bill_db::insert_financial_bill(&db.sqlx_pool, &bill).await?;
                        inserted_bills += 1;
                    } else {
                        discarded_rows += 1;
                    }
                }
                TemplateKind::Transaction => {
                    if let Some(txn) = build_transaction_from_fields(
                        &field_values,
                        email_id,
                        received_ts,
                        template.id,
                    ) {
                        txn_db::insert_financial_transaction(&db.sqlx_pool, &txn).await?;
                        inserted_transactions += 1;
                    } else {
                        discarded_rows += 1;
                    }
                }
            }
        }
    }

    Ok(FinancialExtractionRunResponse {
        templates_scanned: load_template_count(&db, credential_id).await?,
        emails_scanned,
        inserted_bills,
        inserted_transactions,
        discarded_rows,
    })
}

async fn load_templates(
    db: &Arc<Database>,
    credential_id: Option<i64>,
) -> Result<Vec<TemplateRuntime>> {
    let rows = if let Some(cred) = credential_id {
        sqlx::query(
            "SELECT DISTINCT t.id, t.template_type, t.template_body
             FROM financial_extraction_templates t
             JOIN financial_template_email_links l ON l.template_id = t.id
             JOIN emails e ON e.id = l.email_id
             WHERE t.status = 'active'
               AND t.data_source_type = 'email'
               AND e.credential_id = ?",
        )
        .bind(cred)
        .fetch_all(&db.sqlx_pool)
        .await?
    } else {
        sqlx::query(
            "SELECT t.id, t.template_type, t.template_body
             FROM financial_extraction_templates t
             WHERE t.status = 'active'
               AND t.data_source_type = 'email'",
        )
        .fetch_all(&db.sqlx_pool)
        .await?
    };

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let template_type: String = row.try_get("template_type")?;
        let kind = match template_type.as_str() {
            "bill" => TemplateKind::Bill,
            "transaction" => TemplateKind::Transaction,
            _ => continue,
        };
        let vars = sqlx::query(
            "SELECT placeholder_name, target_field
             FROM financial_template_variables
             WHERE template_id = ?
             ORDER BY id ASC",
        )
        .bind(id)
        .fetch_all(&db.sqlx_pool)
        .await?;
        let placeholder_to_field = vars
            .into_iter()
            .filter_map(|v| {
                let p: Option<String> = v.try_get("placeholder_name").ok();
                let f: Option<String> = v.try_get("target_field").ok();
                match (p, f) {
                    (Some(p), Some(f)) => Some((p, f)),
                    _ => None,
                }
            })
            .collect::<HashMap<_, _>>();
        out.push(TemplateRuntime {
            id,
            template_body: row.try_get("template_body")?,
            kind,
            placeholder_to_field,
        });
    }
    Ok(out)
}

async fn load_template_count(db: &Arc<Database>, credential_id: Option<i64>) -> Result<usize> {
    let count: i64 = if let Some(cred) = credential_id {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT t.id)
             FROM financial_extraction_templates t
             JOIN financial_template_email_links l ON l.template_id = t.id
             JOIN emails e ON e.id = l.email_id
             WHERE t.status = 'active'
               AND t.data_source_type = 'email'
               AND e.credential_id = ?",
        )
        .bind(cred)
        .fetch_one(&db.sqlx_pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM financial_extraction_templates
             WHERE status = 'active'
               AND data_source_type = 'email'",
        )
        .fetch_one(&db.sqlx_pool)
        .await?
    };
    Ok(count.max(0) as usize)
}

async fn load_linked_emails_for_template(
    db: &Arc<Database>,
    template_id: i64,
    credential_id: Option<i64>,
) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
    let rows = if let Some(cred) = credential_id {
        sqlx::query(
            "SELECT e.id, e.date_received, e.subject, e.body_text, e.body_html
             FROM financial_template_email_links l
             JOIN emails e ON e.id = l.email_id
             WHERE l.template_id = ?
               AND e.credential_id = ?
             ORDER BY e.date_received DESC",
        )
        .bind(template_id)
        .bind(cred)
        .fetch_all(&db.sqlx_pool)
        .await?
    } else {
        sqlx::query(
            "SELECT e.id, e.date_received, e.subject, e.body_text, e.body_html
             FROM financial_template_email_links l
             JOIN emails e ON e.id = l.email_id
             WHERE l.template_id = ?
             ORDER BY e.date_received DESC",
        )
        .bind(template_id)
        .fetch_all(&db.sqlx_pool)
        .await?
    };
    Ok(rows)
}

fn build_transaction_from_fields(
    fields: &HashMap<String, String>,
    email_id: i64,
    date_received_ms: i64,
    template_id: i64,
) -> Option<FinancialTransaction> {
    let extracted_at = chrono::Utc::now().timestamp_millis();
    let default_date = chrono::DateTime::from_timestamp_millis(date_received_ms)
        .map(|dt| dt.date_naive().format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| {
            chrono::Utc::now()
                .date_naive()
                .format("%Y-%m-%d")
                .to_string()
        });

    let amount = parse_amount(fields.get("amount")?)?;
    let date = fields
        .get("transaction-date")
        .and_then(|v| parse_date(v))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or(default_date);
    Some(FinancialTransaction {
        id: 0,
        data_source_type: DataSourceType::Email,
        data_source_id: email_id.to_string(),
        amount: amount.abs(),
        currency: fields
            .get("currency")
            .cloned()
            .unwrap_or_else(|| "USD".to_string()),
        transaction_date: date,
        category: Some(TransactionCategory::Expense),
        payer: TransactionParty { vendor_id: None },
        payee: TransactionParty { vendor_id: None },
        status: TransactionStatus::Paid,
        source_file: None,
        extracted_at,
        notes: Some(format!("template_id={template_id};kind=transaction")),
        transaction_reference: fields.get("transaction-reference").cloned(),
    })
}

fn build_bill_from_fields(
    fields: &HashMap<String, String>,
    email_id: i64,
    date_received_ms: i64,
) -> Option<Bill> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let now_s = chrono::Utc::now().timestamp();
    let amount = parse_amount(fields.get("total-amount")?)?.abs();

    let issued_date_raw = fields.get("issued-date").cloned();
    let issued_date = issued_date_raw
        .as_ref()
        .and_then(|v| parse_date(v))
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp_millis());

    let due_date_raw = fields.get("due-date").cloned();
    let fallback_due = chrono::DateTime::from_timestamp_millis(date_received_ms)
        .map(|dt| dt.date_naive())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp_millis());
    let due_date = due_date_raw
        .as_ref()
        .and_then(|v| parse_date(v))
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp_millis())
        .or(fallback_due);

    let status = match due_date {
        Some(v) if v < now_ms => BillStatus::Overdue,
        _ => BillStatus::Unpaid,
    };

    Some(Bill {
        id: 0,
        data_source_type: DataSourceType::Email,
        data_source_id: email_id.to_string(),
        document_type: FinancialDocumentType::Bill,
        status,
        issuer_vendor_id: None,
        document_reference: fields.get("document-reference").cloned(),
        total_amount: Some(amount),
        currency: Some(
            fields
                .get("currency")
                .cloned()
                .unwrap_or_else(|| "USD".to_string()),
        ),
        issued_date_raw,
        issued_date,
        due_date_raw,
        due_date,
        billing_period_start_raw: None,
        billing_period_start: None,
        billing_period_end_raw: None,
        billing_period_end: None,
        created_at: now_s,
        updated_at: now_s,
    })
}

fn preferred_body_text(body_text: Option<&str>, body_html: Option<&str>) -> String {
    let text = body_text.unwrap_or_default().trim();
    if !text.is_empty() {
        return text.to_string();
    }
    body_html.unwrap_or_default().to_string()
}
