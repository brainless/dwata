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
    Paid,
    Cancelled,
    Refunded,
}

/// Financial transaction extracted from documents
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Transaction {
    pub id: i64,
    pub data_source_type: DataSourceType,
    pub data_source_id: String,
    pub amount: f64,
    pub currency: String,
    pub transaction_date_raw: Option<String>,
    pub transaction_date: Option<i64>,
    pub status: TransactionStatus,
    pub payer_organisation_id: Option<i64>,
    pub payee_organisation_id: Option<i64>,
    pub transaction_reference: Option<String>,
    pub bill_id: Option<i64>,
    pub source_file: Option<String>,
    pub extracted_at: i64,
}
