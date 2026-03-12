use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum OrderStatus {
    Placed,
    Confirmed,
    Shipped,
    OutForDelivery,
    Delivered,
    Cancelled,
    Returned,
    Refunded,
    Unknown,
}

/// An e-commerce or product order extracted from an email.
/// Distinct from a Transaction (payment event) — an order may contain
/// multiple items, a tracking number, and a delivery status.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Order {
    pub id: i64,
    /// The seller / merchant organisation
    pub organisation_id: Option<i64>,
    pub order_reference: Option<String>,
    /// Raw date string from source document
    pub order_date_raw: Option<String>,
    pub order_date: Option<i64>,
    pub status: OrderStatus,
    pub total_amount: Option<f64>,
    pub currency: Option<String>,
    /// Free-form list of item names/descriptions found in the email
    pub items: Vec<String>,
    pub tracking_number: Option<String>,
    /// FK to the Transaction that paid for this order, if present in the same email
    pub transaction_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
