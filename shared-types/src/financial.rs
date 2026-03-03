use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    bill::Bill,
    transaction::{FinancialTransaction, TransactionCategory},
};

/// Financial summary/overview
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialSummary {
    pub total_income: f64,
    pub total_expenses: f64,
    pub net_balance: f64,
    pub pending_bills: i32,
    pub overdue_payments: i32,
    pub currency: String,
    pub period_start: String,
    pub period_end: String,
}

/// Financial extraction source summary
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialExtractionSummary {
    pub source_count: i64,
    pub transaction_count: i64,
    pub last_extracted_at: Option<i64>,
}

/// Financial health metrics
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialHealth {
    pub summary: FinancialSummary,
    pub recent_transactions: Vec<FinancialTransaction>,
    pub upcoming_bills: Vec<Bill>,
    pub category_breakdown: Vec<CategoryBreakdown>,
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
