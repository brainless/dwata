use crate::config::ApiConfig;
use crate::database::{financial_bills as bill_db, financial_transactions as txn_db, Database};
use anyhow::Result;
use dwata_agents::storage::{AgentStorage, InMemoryAgentStorage, Session};
use dwata_agents::{
    extract_values_from_email_with_values, parse_amount, parse_date, simple_email_content,
    LlmTemplateVariableExtractorAgent, TemplateEmailContent, TemplateVariableType,
};
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use serde::Serialize;
use shared_types::{
    Bill, BillStatus, DataSourceType, FinancialDocumentType, Transaction, TransactionCategory,
    TransactionStatus,
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
    kind: TemplateKind,
}

pub async fn extract_financial_from_templates(
    db: Arc<Database>,
    _config: &ApiConfig,
    credential_id: Option<i64>,
) -> Result<FinancialExtractionRunResponse> {
    let llm_client = Arc::new(OllamaClient::new()?);
    let storage = Arc::new(InMemoryAgentStorage::new());

    let mut templates = load_templates(&db, credential_id).await?;
    let mut emails_scanned = 0usize;
    let mut inserted_bills = 0usize;
    let mut inserted_transactions = 0usize;
    let mut discarded_rows = 0usize;

    for template in templates.drain(..) {
        let var_type = match template.kind {
            TemplateKind::Bill => TemplateVariableType::Bill,
            TemplateKind::Transaction => TemplateVariableType::Transaction,
        };

        let email_rows = load_linked_emails_for_template(&db, template.id, credential_id).await?;
        for email in email_rows {
            emails_scanned += 1;

            let subject: Option<String> = email.try_get("subject")?;
            let body_text: Option<String> = email.try_get("body_text")?;
            let body_html: Option<String> = email.try_get("body_html")?;
            let email_id: i64 = email.try_get("id")?;
            let received_ts: i64 = email.try_get("date_received")?;

            let simple = simple_email_content(
                subject.as_deref(),
                body_text.as_deref(),
                body_html.as_deref(),
            );

            let session_id = storage
                .create_session(Session {
                    id: None,
                    agent_type: "llm-template-variable-extractor".to_string(),
                    objective: format!("Extract {:?} variables", var_type),
                    context_data: None,
                    status: "running".to_string(),
                    result: None,
                })
                .await?;

            let agent = LlmTemplateVariableExtractorAgent::new(
                llm_client.clone(),
                storage.clone(),
                var_type,
                simple.subject.clone(),
                simple.body.clone(),
            );

            let extracted_vars = match agent.execute(session_id).await {
                Ok(v) => v,
                Err(_) => {
                    discarded_rows += 1;
                    continue;
                }
            };

            if extracted_vars.variables.is_empty() {
                discarded_rows += 1;
                continue;
            }

            let fields = extract_values_from_email_with_values(
                &extracted_vars.variables,
                &TemplateEmailContent {
                    subject: simple.subject,
                    body: simple.body,
                },
            );

            if fields.is_empty() {
                discarded_rows += 1;
                continue;
            }

            match template.kind {
                TemplateKind::Bill => {
                    if let Some(bill) = build_bill_from_fields(&fields, email_id, received_ts) {
                        bill_db::insert_financial_bill(&db.sqlx_pool, &bill).await?;
                        inserted_bills += 1;
                    } else {
                        discarded_rows += 1;
                    }
                }
                TemplateKind::Transaction => {
                    if let Some(txn) =
                        build_transaction_from_fields(&fields, email_id, received_ts, template.id)
                    {
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
            "SELECT DISTINCT t.id, t.template_type
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
            "SELECT t.id, t.template_type
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
        out.push(TemplateRuntime { id, kind });
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
) -> Option<Transaction> {
    let extracted_at = chrono::Utc::now().timestamp_millis();

    let amount = parse_amount(fields.get("amount")?)?;
    let date_raw = fields.get("transaction_date").cloned();
    let date = fields
        .get("transaction_date")
        .and_then(|v| parse_date(v))
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp_millis());
    Some(Transaction {
        id: 0,
        data_source_type: DataSourceType::Email,
        data_source_id: email_id.to_string(),
        amount: amount.abs(),
        currency: fields
            .get("currency")
            .cloned()
            .unwrap_or_else(|| "USD".to_string()),
        transaction_date_raw: date_raw,
        transaction_date: date,
        payer_vendor_id: None,
        payee_vendor_id: None,
        status: TransactionStatus::Paid,
        source_file: None,
        extracted_at,
        bill_id: None,
        transaction_reference: fields
            .get("transaction_reference")
            .cloned()
            .or_else(|| Some(format!("template_id={template_id}"))),
    })
}

fn build_bill_from_fields(
    fields: &HashMap<String, String>,
    email_id: i64,
    date_received_ms: i64,
) -> Option<Bill> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let now_s = chrono::Utc::now().timestamp();
    let amount = parse_amount(fields.get("total_amount")?)?.abs();

    let issued_date_raw = fields.get("issued_date").cloned();
    let issued_date = issued_date_raw
        .as_ref()
        .and_then(|v| parse_date(v))
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp_millis());

    let due_date_raw = fields.get("due_date").cloned();
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
        category: None,
        issuer_vendor_id: None,
        document_reference: fields.get("document_reference").cloned(),
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
