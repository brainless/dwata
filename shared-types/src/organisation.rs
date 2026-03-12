use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The role an organisation plays, especially in financial contexts.
/// An organisation can play multiple roles across different interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum OrganisationRole {
    /// A for-profit company or business
    Business,
    /// A financial institution (bank, credit union)
    Bank,
    /// An insurance provider
    Insurer,
    /// A payment processor or financial platform (Stripe, PayPal, Razorpay)
    PaymentPlatform,
    /// An employer or staffing agency
    Employer,
    /// A utility provider (electricity, water, gas, internet)
    Utility,
    /// A subscription-based service provider
    ServiceProvider,
    /// A government body or public institution
    Government,
    /// An educational institution
    Educational,
    /// A non-profit or charity
    NonProfit,
    /// Could not be determined
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Organisation {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub role: Option<OrganisationRole>,
    pub location_id: Option<i64>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateOrganisationRequest {
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub role: Option<OrganisationRole>,
    pub location_id: Option<i64>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateOrganisationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub role: Option<OrganisationRole>,
    pub location_id: Option<i64>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct OrganisationsResponse {
    pub organisations: Vec<Organisation>,
}
