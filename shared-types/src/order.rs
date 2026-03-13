use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Order status options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
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

/// A product or e-commerce order extracted from emails
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Order {
    pub id: i64,
    /// FK to the Organisation that is the seller/merchant
    pub organisation_id: Option<i64>,
    pub order_reference: Option<String>,
    /// Raw date string exactly as it appeared in the email
    pub order_date_raw: Option<String>,
    /// Parsed UTC timestamp in milliseconds
    pub order_date: Option<i64>,
    pub status: Option<OrderStatus>,
    pub total_amount: Option<f64>,
    pub currency: Option<String>,
    /// List of product/item names or descriptions from the email
    #[ts(skip)]
    pub items: serde_json::Value, // Array of strings
    pub tracking_number: Option<String>,
    /// FK to the Transaction that paid for this order
    pub transaction_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct OrdersResponse {
    pub orders: Vec<Order>,
}
