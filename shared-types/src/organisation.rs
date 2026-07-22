use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The role an organisation plays, especially in financial contexts.
/// An organisation can play multiple roles across different interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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

impl std::fmt::Display for OrganisationRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrganisationRole::Business => write!(f, "business"),
            OrganisationRole::Bank => write!(f, "bank"),
            OrganisationRole::Insurer => write!(f, "insurer"),
            OrganisationRole::PaymentPlatform => write!(f, "payment-platform"),
            OrganisationRole::Employer => write!(f, "employer"),
            OrganisationRole::Utility => write!(f, "utility"),
            OrganisationRole::ServiceProvider => write!(f, "service-provider"),
            OrganisationRole::Government => write!(f, "government"),
            OrganisationRole::Educational => write!(f, "educational"),
            OrganisationRole::NonProfit => write!(f, "non-profit"),
            OrganisationRole::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organisation {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    /// Primary contact or billing email for this organisation.
    pub email: Option<String>,
    /// All roles this organisation plays; stored in a separate organisation_roles table.
    pub roles: Vec<OrganisationRole>,
    pub location_id: Option<i64>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    /// LLM-generated summary for BM25 search during future extraction passes.
    /// e.g. "streaming service, monthly billing via credit card, support at help@netflix.com"
    pub search_summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Organisation with email count statistics derived from the emails table.
#[derive(Debug, Serialize)]
pub struct OrganisationWithCounts {
    #[serde(flatten)]
    pub organisation: Organisation,
    /// Emails where this organisation is the sender (`from_address` match).
    pub received_count: i64,
    /// Emails where this organisation appears in `to_addresses` or `cc_addresses`.
    pub in_to_count: i64,
}

#[derive(Debug, Serialize)]
pub struct OrganisationsWithCountsResponse {
    pub organisations: Vec<OrganisationWithCounts>,
}
