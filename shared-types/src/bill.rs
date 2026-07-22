use serde::{Deserialize, Serialize};

use crate::transaction::{DataSourceType, TransactionCategory};

/// Status specific to a financial document (bill, invoice, statement).
/// Distinct from TransactionStatus which tracks payment events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BillStatus {
    Received,
    Unpaid,
    Paid,
    Overdue,
    Cancelled,
}

/// A financial document (bill, invoice, receipt, statement) extracted from an email or file.
/// One Bill may be the source for zero (unpaid) or more Transactions.
///
/// ## Date Column Conventions
///
/// Every date has two columns:
/// - `{field}_raw`  — `TEXT` — the exact date string as it appeared in the source document
///                    (e.g., "15 Jan 2025", "January 15th, 2025", "15/01/25")
/// - `{field}`      — `BIGINT` — parsed UTC timestamp in milliseconds since Unix epoch.
///                    For date-only values, use 00:00:00 UTC for that calendar day.
///                    Nullable when parsing fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bill {
    pub id: i64,
    pub data_source_type: DataSourceType,
    pub data_source_id: String,
    pub status: BillStatus,
    pub category: Option<TransactionCategory>,
    /// FK to the Organisation that issued this bill
    pub issuer_organisation_id: Option<i64>,
    /// FK to the Subscription this bill belongs to (if recurring)
    pub subscription_id: Option<i64>,
    pub document_reference: Option<String>,
    pub total_amount: Option<f64>,
    pub currency: Option<String>,

    /// Date the bill or invoice was generated or issued by the vendor.
    /// Distinct from due_date (when payment is expected) and billing_period (service window).
    /// SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
    pub issued_date_raw: Option<String>,
    pub issued_date: Option<i64>,

    /// Date by which payment must be made.
    /// SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
    pub due_date_raw: Option<String>,
    pub due_date: Option<i64>,

    /// Start and end of the billing or service period this bill covers.
    /// SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
    pub billing_period_start_raw: Option<String>,
    pub billing_period_start: Option<i64>,
    pub billing_period_end_raw: Option<String>,
    pub billing_period_end: Option<i64>,

    pub created_at: i64,
    pub updated_at: i64,
}
