use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Order status options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// A single line item within an order.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderItem {
    pub name: String,
    pub quantity: Option<i32>,
    pub unit_price: Option<f64>,
}

/// A product or e-commerce order extracted from emails
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub items: Vec<OrderItem>,
    pub tracking_number: Option<String>,
    /// FK to the Transaction that paid for this order
    pub transaction_id: Option<i64>,
    /// FK to the email this order was extracted from.
    pub source_email_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct OrdersResponse {
    pub orders: Vec<Order>,
}
