use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::bill::FinancialDocumentType;

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

/// Lifecycle status for user-in-the-loop transaction enrichment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum EnrichmentStatus {
    RawExtracted,
    PartiallyResolved,
    UserConfirmed,
    FullyResolved,
}

/// Explicitly tracked unresolved fields requiring user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum UnresolvedField {
    PayerIdentity,
    PayeeIdentity,
    Category,
    TransactionReference,
    TransactionDate,
    Currency,
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

/// Strongly typed transaction endpoint. Always present for both payer and payee.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TransactionParty {
    /// Canonical vendor reference. Null means unresolved.
    pub vendor_id: Option<i64>,
}

/// Financial transaction extracted from documents
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialTransaction {
    pub id: i64,
    pub data_source_type: DataSourceType,
    pub data_source_id: String,
    pub financial_document_id: Option<i64>,
    pub document_type: FinancialDocumentType,
    pub description: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub transaction_date: String,
    pub category: Option<TransactionCategory>,
    pub payer: TransactionParty,
    pub payee: TransactionParty,
    pub status: TransactionStatus,
    pub enrichment_status: EnrichmentStatus,
    pub unresolved_items: Vec<UnresolvedField>,
    pub source_file: Option<String>,
    pub extracted_at: i64,
    pub notes: Option<String>,
    pub transaction_reference: Option<String>,
}
