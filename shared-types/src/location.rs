use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Location {
    pub id: i64,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    /// ISO 3166-1 alpha-2 or alpha-3 country code
    pub country_code: Option<String>,
    pub postal_code: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
