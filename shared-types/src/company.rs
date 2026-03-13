use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Company {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location_id: Option<i64>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CompaniesResponse {
    pub companies: Vec<Company>,
}
