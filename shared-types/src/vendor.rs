use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Vendor type for transaction parties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum VendorType {
    SelfUser,
    SelfBusiness,
    FinancialInstrument,
    Merchant,
    Employer,
    Bank,
    Individual,
    Platform,
    Unknown,
}

/// Transaction vendor entity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Vendor {
    pub id: i64,
    pub vendor_type: VendorType,
    pub vendor_name: String,
    pub vendor_external_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
