use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum BillingCycle {
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum SubscriptionStatus {
    Active,
    Paused,
    Cancelled,
    Expired,
    Trial,
    Unknown,
}

/// A recurring subscription to a service. One subscription generates many Bills over its lifetime.
/// Bills reference back via Bill.subscription_id.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Subscription {
    pub id: i64,
    /// The organisation providing the service
    pub organisation_id: Option<i64>,
    /// Human-readable service name, e.g. "Netflix", "GitHub Pro", "AWS"
    pub service_name: String,
    /// Membership/plan tier, e.g. "Premium", "Pro", "Family", "Standard"
    pub plan_name: Option<String>,
    pub billing_cycle: Option<BillingCycle>,
    /// Recurring charge amount (numeric)
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub status: SubscriptionStatus,
    /// Raw date string from source document
    pub start_date_raw: Option<String>,
    pub start_date: Option<i64>,
    /// Raw date string from source document
    pub next_billing_date_raw: Option<String>,
    pub next_billing_date: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
