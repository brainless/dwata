use crate::financial_patterns::{create_financial_patterns, FinancialPattern};
use chrono::Utc;
use shared_types::FinancialTransaction;

pub struct FinancialPatternExtractor {
    patterns: Vec<FinancialPattern>,
}

impl FinancialPatternExtractor {
    pub fn new() -> Self {
        Self {
            patterns: create_financial_patterns(),
        }
    }

    pub fn extract_from_text(&self, text: &str) -> Vec<FinancialTransaction> {
        let mut transactions = Vec::new();

        for pattern in &self.patterns {
            if let Some(transaction) = pattern.extract(text) {
                let mut txn = transaction;
                txn.id = 0;
                txn.extracted_at = Utc::now().timestamp();
                transactions.push(txn);
            }
        }

        transactions
    }

    pub fn extract_from_email(&self, subject: &str, body_text: &str) -> Vec<FinancialTransaction> {
        let text = format!("{}\n\n{}", subject, body_text);
        self.extract_from_text(&text)
    }
}

impl Default for FinancialPatternExtractor {
    fn default() -> Self {
        Self::new()
    }
}
