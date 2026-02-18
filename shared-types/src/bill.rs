use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::transaction::{DataSourceType, FinancialDocumentType, TransactionStatus};

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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialDocument {
    pub id: i64,
    pub data_source_type: DataSourceType,
    pub data_source_id: String,
    pub document_type: FinancialDocumentType,
    pub status: TransactionStatus,
    pub issuer_vendor_id: Option<i64>,
    pub document_reference: Option<String>,
    pub due_date: Option<String>,
    pub billing_period_start: Option<String>,
    pub billing_period_end: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialDocumentSubject {
    pub id: i64,
    pub financial_document_id: i64,
    pub kind: ServiceIdentifierKind,
    pub value: String,
    pub masked_value: Option<String>,
    pub is_primary: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
