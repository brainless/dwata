use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::transaction::DataSourceType;

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

/// Status specific to a financial document (bill, invoice, statement).
/// Distinct from TransactionStatus which tracks payment events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum BillStatus {
    Received,
    Unpaid,
    Paid,
    Overdue,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceIdentifierKind {
    PhoneNumber,
    AccountNumber,
    PolicyNumber,
    MeterNumber,
    SubscriptionId,
    ContractId,
    Other,
}

/// A financial document (bill, invoice, receipt, statement) extracted from an email or file.
/// One Bill may be the source for zero (unpaid) or more FinancialTransactions.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Bill {
    pub id: i64,
    pub data_source_type: DataSourceType,
    pub data_source_id: String,
    pub document_type: FinancialDocumentType,
    pub status: BillStatus,
    pub issuer_vendor_id: Option<i64>,
    pub document_reference: Option<String>,
    pub total_amount: Option<f64>,
    pub currency: Option<String>,
    pub due_date: Option<String>,
    pub billing_period_start: Option<String>,
    pub billing_period_end: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BillSubject {
    pub id: i64,
    pub bill_id: i64,
    pub kind: ServiceIdentifierKind,
    pub value: String,
    pub masked_value: Option<String>,
    pub is_primary: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
