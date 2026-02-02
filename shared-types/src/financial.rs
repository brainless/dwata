use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Financial document types that can be extracted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum FinancialDocumentType {
    Invoice,
    Bill,
    BankStatement,
    Receipt,
    TaxDocument,
    PaymentConfirmation,
}

/// Financial transaction extracted from documents
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialTransaction {
    pub id: i64,

    // Source tracking (agnostic to source type)
    pub source_type: String, // 'email', 'document', 'chat', 'file', etc.
    pub source_id: String,   // ID in the source system

    // Transaction data
    pub document_type: FinancialDocumentType,
    pub description: String,
    pub amount: f64,
    pub currency: String,
    pub transaction_date: String,

    // Additional fields
    pub category: Option<TransactionCategory>,
    pub vendor: Option<String>,
    pub status: TransactionStatus,

    // Metadata
    pub source_file: Option<String>,
    pub extracted_at: i64,
    pub notes: Option<String>,
}

/// Category for financial transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionCategory {
    Income,
    Expense,
    Investment,
    Tax,
    Utility,
    Subscription,
    Entertainment,
    Travel,
    Healthcare,
    Education,
    Other,
}

/// Status of financial transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionStatus {
    Pending,
    Paid,
    Overdue,
    Cancelled,
    Refunded,
}

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

/// Financial health metrics
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialHealth {
    pub summary: FinancialSummary,
    pub recent_transactions: Vec<FinancialTransaction>,
    pub upcoming_bills: Vec<FinancialTransaction>,
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

/// Financial pattern for extracting transactions
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialPattern {
    pub id: i64,
    pub name: String,
    pub regex_pattern: String,
    pub description: Option<String>,
    pub document_type: String,
    pub status: String,
    pub confidence: f32,
    pub amount_group: usize,
    pub vendor_group: Option<usize>,
    pub date_group: Option<usize>,
    pub is_default: bool,
    pub is_active: bool,
    pub match_count: i32,
    pub last_matched_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
