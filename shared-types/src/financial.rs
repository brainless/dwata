use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::bill::Bill;
use crate::transaction::TransactionCategory;

/// Financial extraction source summary
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialExtractionSummary {
    pub source_count: i64,
    pub transaction_count: i64,
    pub last_extracted_at: Option<i64>,
}

/// Breakdown by category
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CategoryBreakdown {
    pub category: TransactionCategory,
    pub amount: f64,
    pub percentage: f64,
    pub transaction_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialPagination {
    pub page: usize,
    pub limit: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ListFinancialBillsResponse {
    pub bills: Vec<Bill>,
    pub pagination: FinancialPagination,
}
