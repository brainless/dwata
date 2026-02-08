use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Company {
    pub id: i64,
    pub extraction_job_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    pub confidence: Option<f32>,
    pub requires_review: bool,
    pub is_confirmed: bool,
    pub is_duplicate: bool,
    pub merged_into_company_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCompanyRequest {
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCompanyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    pub is_confirmed: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CompaniesResponse {
    pub companies: Vec<Company>,
}
