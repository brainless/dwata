use crate::financial_extractor::types::{SavePatternParams, TestPatternParams};
use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use shared_types::{
    DataSourceType, FinancialDocumentType, FinancialTransaction, TransactionStatus,
};

pub struct DwataToolExecutor {
    conn: Arc<Mutex<Connection>>,
    email_content: String,
}

impl DwataToolExecutor {
    pub fn new(conn: Arc<Mutex<Connection>>, email_content: String) -> Self {
        Self { conn, email_content }
    }

    pub async fn test_pattern(&self, params: TestPatternParams) -> Result<Vec<FinancialTransaction>> {
        let regex = regex::Regex::new(&params.regex_pattern)?;
        let mut transactions = Vec::new();

        for caps in regex.captures_iter(&self.email_content) {
            let amount = if let Some(amount_match) = caps.get(params.amount_group) {
                let amount_str = amount_match
                    .as_str()
                    .replace(',', "")
                    .replace('$', "")
                    .trim()
                    .to_string();
                amount_str.parse::<f64>().ok()
            } else {
                None
            };

            let source_vendor = params
                .source_vendor_group
                .and_then(|g| caps.get(g))
                .map(|m| m.as_str().trim().to_string());
            let destination_vendor = params
                .destination_vendor_group
                .and_then(|g| caps.get(g))
                .map(|m| m.as_str().trim().to_string());
            let vendor = destination_vendor
                .clone()
                .or_else(|| source_vendor.clone());

            let transaction_date = params.date_group.and_then(|g| caps.get(g)).map(|m| {
                m.as_str()
                    .trim()
                    .to_string()
            });
            let reference = params
                .reference_group
                .and_then(|g| caps.get(g))
                .map(|m| m.as_str().trim().to_string());

            if let Some(amount) = amount {
                transactions.push(FinancialTransaction {
                    id: 0,
                    data_source_type: DataSourceType::Email,
                    data_source_id: "test".to_string(),
                    document_type: FinancialDocumentType::Receipt,
                    description: caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
                    amount,
                    currency: "USD".to_string(),
                    transaction_date: transaction_date.unwrap_or_else(|| {
                        chrono::Utc::now().format("%Y-%m-%d").to_string()
                    }),
                    category: None,
                    vendor,
                    source_vendor_id: None,
                    destination_vendor_id: None,
                    status: TransactionStatus::Pending,
                    source_file: None,
                    extracted_at: chrono::Utc::now().timestamp(),
                    notes: None,
                    transaction_reference: reference,
                });
            }
        }

        Ok(transactions)
    }

    pub async fn save_pattern(&self, params: SavePatternParams) -> Result<i64> {
        if std::env::var("DWATA_SKIP_PATTERN_SAVE").ok().as_deref() == Some("1") {
            return Ok(0);
        }

        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO financial_patterns
             (name, regex_pattern, description, document_type, status, confidence,
              amount_group, vendor_group, source_vendor_group, destination_vendor_group,
              date_group, reference_group, is_default, is_active,
              match_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, false, true, 0, ?, ?)",
            rusqlite::params![
                params.name,
                params.regex_pattern,
                params.description,
                params.document_type,
                params.status,
                params.confidence,
                params.amount_group as i64,
                None::<i64>,
                params.source_vendor_group.map(|v| v as i64),
                params.destination_vendor_group.map(|v| v as i64),
                params.date_group.map(|v| v as i64),
                params.reference_group.map(|v| v as i64),
                now,
                now,
            ],
        )?;

        let pattern_id = conn.last_insert_rowid();
        Ok(pattern_id)
    }
}
