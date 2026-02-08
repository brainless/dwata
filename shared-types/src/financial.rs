use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Data source type for extracted transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum DataSourceType {
    Email,
    Imap,
    BankStatement,
    CreditCardStatement,
    BankFeed,
    CsvUpload,
    Manual,
    Unknown,
}

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
    pub data_source_type: DataSourceType,
    pub data_source_id: String,   // ID in the source system

    // Transaction data
    pub document_type: FinancialDocumentType,
    pub description: String,
    pub amount: f64,
    pub currency: String,
    pub transaction_date: String,

    // Additional fields
    pub category: Option<TransactionCategory>,
    pub vendor: Option<String>,
    pub source_vendor_id: Option<i64>,
    pub destination_vendor_id: Option<i64>,
    pub status: TransactionStatus,

    // Metadata
    pub source_file: Option<String>,
    pub extracted_at: i64,
    pub notes: Option<String>,
    pub transaction_reference: Option<String>,
}

/// Vendor type for transaction parties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionVendorType {
    Bank,
    Business,
    Employee,
    Individual,
    Platform,
    Unknown,
}

/// Transaction vendor entity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TransactionVendor {
    pub id: i64,
    pub vendor_type: TransactionVendorType,
    pub vendor_name: String,
    pub vendor_external_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
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

/// Financial extraction source summary
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialExtractionSummary {
    pub source_count: i64,
    pub transaction_count: i64,
    pub last_extracted_at: Option<i64>,
}

/// Financial extraction attempt details
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialExtractionAttempt {
    pub id: i64,
    pub source_type: String,
    pub source_account_id: i64,
    pub attempted_at: i64,
    pub total_items_scanned: i64,
    pub transactions_extracted: i64,
    pub status: String,
    pub error_message: Option<String>,
}

/// Response for extraction attempt history
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialExtractionAttemptsResponse {
    pub attempts: Vec<FinancialExtractionAttempt>,
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
    pub sender_email: Option<String>,
    pub document_type: String,
    pub status: String,
    pub confidence: f32,
    pub amount_group: usize,
    pub vendor_group: Option<usize>,
    pub source_vendor_group: Option<usize>,
    pub destination_vendor_group: Option<usize>,
    pub date_group: Option<usize>,
    pub reference_group: Option<usize>,
    pub is_default: bool,
    pub is_active: bool,
    pub match_count: i32,
    pub last_matched_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
