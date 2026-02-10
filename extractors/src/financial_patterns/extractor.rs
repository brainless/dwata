use crate::financial_patterns::{create_financial_patterns, FinancialPattern};
use chrono::Utc;
use regex::Regex;
use shared_types::{
    DataSourceType, FinancialDocumentType, FinancialTransaction, TransactionCategory,
    TransactionStatus,
};
use crate::financial_patterns::normalize_currency;

pub struct FinancialPatternExtractor {
    patterns: Vec<CompiledPattern>,
}

struct CompiledPattern {
    id: i64,
    name: String,
    regex: Regex,
    document_type: String,
    status: String,
    amount_group: usize,
    vendor_group: Option<usize>,
    source_vendor_group: Option<usize>,
    destination_vendor_group: Option<usize>,
    date_group: Option<usize>,
    reference_group: Option<usize>,
    currency_group: Option<usize>,
}

impl FinancialPatternExtractor {
    pub fn new() -> Self {
        Self {
            patterns: create_financial_patterns()
                .into_iter()
                .map(|p| CompiledPattern {
                    id: 0,
                    name: p.name,
                    regex: p.regex,
                    document_type: format!("{:?}", p.transaction_type)
                        .to_lowercase()
                        .replace("_", "-"),
                    status: format!("{:?}", p.status).to_lowercase(),
                    amount_group: p.amount_group,
                    vendor_group: p.vendor_group,
                    source_vendor_group: p.source_vendor_group,
                    destination_vendor_group: p.destination_vendor_group,
                    date_group: p.date_group,
                    reference_group: p.reference_group,
                    currency_group: p.currency_group,
                })
                .collect(),
        }
    }

    pub fn from_patterns(db_patterns: Vec<shared_types::FinancialPattern>) -> anyhow::Result<Self> {
        let mut patterns = Vec::new();

        for pattern in db_patterns {
            let regex = Regex::new(&pattern.regex_pattern)?;

            patterns.push(CompiledPattern {
                id: pattern.id,
                name: pattern.name,
                regex,
                document_type: pattern.document_type,
                status: pattern.status,
                amount_group: pattern.amount_group,
                vendor_group: pattern.vendor_group,
                source_vendor_group: pattern.source_vendor_group,
                destination_vendor_group: pattern.destination_vendor_group,
                date_group: pattern.date_group,
                reference_group: pattern.reference_group,
                currency_group: pattern.currency_group,
            });
        }

        Ok(Self { patterns })
    }

    pub fn extract_from_text(&self, text: &str) -> Vec<(FinancialTransaction, i64)> {
        let mut results = Vec::new();

        for pattern in &self.patterns {
            if let Some(caps) = pattern.regex.captures(text) {
                if let Some(transaction) = self.extract_transaction(&pattern, &caps) {
                    results.push((transaction, pattern.id));
                }
            }
        }

        results
    }

    pub fn extract_from_email(&self, subject: &str, body_text: &str) -> Vec<FinancialTransaction> {
        let text = format!("{}\n\n{}", subject, body_text);
        self.extract_from_text(&text)
            .into_iter()
            .map(|(mut txn, _)| {
                txn.id = 0;
                txn.extracted_at = Utc::now().timestamp();
                txn
            })
            .collect()
    }

    fn extract_transaction(
        &self,
        pattern: &CompiledPattern,
        captures: &regex::Captures,
    ) -> Option<FinancialTransaction> {
        let amount_str = captures.get(pattern.amount_group)?.as_str();
        let amount = self.parse_amount(amount_str)?;

        let source_vendor = pattern
            .source_vendor_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str().trim().to_string());

        let destination_vendor = pattern
            .destination_vendor_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str().trim().to_string());

        if source_vendor.is_none() && destination_vendor.is_none() {
            return None;
        }

        let vendor = destination_vendor.clone().or_else(|| source_vendor.clone());

        let transaction_date = pattern
            .date_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str().to_string());

        let category = match pattern.document_type.as_str() {
            "payment-confirmation" => Some(TransactionCategory::Expense),
            "invoice" => Some(TransactionCategory::Income),
            _ => None,
        };

        let currency = pattern
            .currency_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str())
            .and_then(normalize_currency)
            .unwrap_or_else(|| "USD".to_string());

        Some(FinancialTransaction {
            id: 0,
            data_source_type: DataSourceType::Unknown,
            data_source_id: String::new(),
            document_type: self.parse_document_type(&pattern.document_type),
            description: captures.get(0)?.as_str().to_string(),
            amount,
            currency,
            transaction_date: transaction_date
                .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string()),
            category,
            vendor,
            source_vendor_id: None,
            destination_vendor_id: None,
            status: self.parse_status(&pattern.status),
            source_file: None,
            extracted_at: 0,
            notes: None,
            transaction_reference: pattern
                .reference_group
                .and_then(|g| captures.get(g))
                .map(|m| m.as_str().trim().to_string()),
        })
    }

    fn parse_amount(&self, amount_str: &str) -> Option<f64> {
        amount_str
            .replace(',', "")
            .replace("$", "")
            .trim()
            .parse()
            .ok()
    }

    fn parse_document_type(&self, doc_type: &str) -> FinancialDocumentType {
        match doc_type {
            "invoice" => FinancialDocumentType::Invoice,
            "bill" => FinancialDocumentType::Bill,
            "receipt" => FinancialDocumentType::Receipt,
            "payment-confirmation" => FinancialDocumentType::PaymentConfirmation,
            _ => FinancialDocumentType::TaxDocument,
        }
    }

    fn parse_status(&self, status: &str) -> TransactionStatus {
        match status {
            "paid" => TransactionStatus::Paid,
            "pending" => TransactionStatus::Pending,
            "overdue" => TransactionStatus::Overdue,
            "cancelled" => TransactionStatus::Cancelled,
            _ => TransactionStatus::Paid,
        }
    }
}

impl Default for FinancialPatternExtractor {
    fn default() -> Self {
        Self::new()
    }
}
