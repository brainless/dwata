use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Billing cycle options for subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillingCycle {
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    Other,
}

/// A recurring subscription to a service extracted from emails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    /// FK to the Organisation providing the service
    pub organisation_id: Option<i64>,
    /// Human-readable service name, e.g. 'Netflix', 'GitHub Pro', 'AWS'
    pub service_name: String,
    /// Membership or plan tier, e.g. 'Premium', 'Pro', 'Family', 'Standard'
    pub plan_name: Option<String>,
    pub billing_cycle: Option<BillingCycle>,
    /// Recurring charge amount
    pub amount: Option<f64>,
    pub currency: Option<String>,
    /// Raw date string exactly as it appeared in the email
    pub next_billing_date_raw: Option<String>,
    /// Parsed UTC timestamp in milliseconds
    pub next_billing_date: Option<i64>,
    /// Raw date string exactly as it appeared in the email
    pub start_date_raw: Option<String>,
    /// Parsed UTC timestamp in milliseconds
    pub start_date: Option<i64>,
    /// FK to the email this subscription was extracted from.
    pub source_email_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionsResponse {
    pub subscriptions: Vec<Subscription>,
}
