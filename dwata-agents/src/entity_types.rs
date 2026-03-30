use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use shared_types::OrganisationRole;

/// Deserialise a field that the schema declares as a string but small models
/// sometimes emit as a bare JSON number (e.g. `5.41` instead of `"5.41"`).
fn deserialize_number_or_string<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// Per-entity extraction structs (exposed to LLM via tool schema)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedLocation {
    #[schemars(
        description = "Unique positive integer id you assign — other entities reference this"
    )]
    pub id: i64,
    /// Named place, e.g. "Central Park". Omit for pure street addresses.
    pub name: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    #[schemars(description = "ISO 3166-1 alpha-2 or alpha-3 country code if identifiable")]
    pub country_code: Option<String>,
    pub postal_code: Option<String>,
    #[schemars(
        description = "Short BM25-searchable summary capturing relational context, e.g. 'downtown New York, near Central Park'"
    )]
    pub search_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedOrganisation {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: i64,
    pub name: String,
    pub industry: Option<String>,
    pub website: Option<String>,
    /// Primary contact or billing email for this organisation, e.g. billing@netflix.com
    pub email: Option<String>,
    #[schemars(
        description = "All roles this organisation plays. Each value must be one of: Business, Bank, Insurer, PaymentPlatform, Employer, Utility, ServiceProvider, Government, Educational, NonProfit, Unknown"
    )]
    pub roles: Vec<OrganisationRole>,
    #[schemars(description = "FK: id of an ExtractedLocation for this organisation's address")]
    pub location_id: Option<i64>,
    #[schemars(
        description = "Short BM25-searchable summary capturing relational context, e.g. 'streaming service, monthly billing via credit card, support at help@netflix.com'"
    )]
    pub search_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedPerson {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: i64,
    #[schemars(description = "FK: id of the email this person was extracted from")]
    pub email_id: Option<i64>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    #[schemars(description = "FK: id of an ExtractedOrganisation this person belongs to")]
    pub organisation_id: Option<i64>,
    #[schemars(
        description = "Short BM25-searchable summary capturing relational context, e.g. 'engineer at Acme Corp, john@acme.com'"
    )]
    pub search_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedBill {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: i64,
    #[schemars(
        description = "Numeric amount string only — strip currency symbols (e.g. '299.00')"
    )]
    #[serde(default, deserialize_with = "deserialize_number_or_string")]
    pub total_amount: Option<String>,
    #[schemars(description = "ISO currency code or symbol (e.g. INR, USD, $")]
    pub currency: Option<String>,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub issued_date: Option<String>,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub due_date: Option<String>,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub billing_period_start: Option<String>,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub billing_period_end: Option<String>,
    pub document_reference: Option<String>,
    #[schemars(description = "FK: id of an ExtractedOrganisation that issued this bill")]
    pub issuer_organisation_id: Option<i64>,
    #[schemars(
        description = "FK: id of an ExtractedSubscription this bill belongs to (if this is a recurring charge)"
    )]
    pub subscription_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedTransaction {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: i64,
    #[schemars(description = "Numeric amount as a number — strip currency symbols (e.g. 1299.50)")]
    pub amount: f64,
    #[schemars(description = "ISO currency code or symbol (e.g. INR, USD, $")]
    pub currency: String,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub transaction_date: Option<String>,
    pub transaction_reference: Option<String>,
    #[schemars(description = "FK: id of an ExtractedOrganisation who paid")]
    pub payer_organisation_id: Option<i64>,
    #[schemars(description = "FK: id of an ExtractedOrganisation who received payment")]
    pub payee_organisation_id: Option<i64>,
    #[schemars(description = "FK: id of an ExtractedBill this transaction settles")]
    pub bill_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedSubscription {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: i64,
    #[schemars(description = "FK: id of an ExtractedOrganisation providing the service")]
    pub organisation_id: Option<i64>,
    #[schemars(description = "Human-readable service name, e.g. 'Netflix', 'GitHub Pro', 'AWS'")]
    pub service_name: String,
    #[schemars(
        description = "Membership or plan tier, e.g. 'Premium', 'Pro', 'Family', 'Standard'"
    )]
    pub plan_name: Option<String>,
    #[schemars(description = "One of: weekly, monthly, quarterly, semi_annual, annual, other")]
    pub billing_cycle: Option<String>,
    #[schemars(description = "Recurring charge amount as a number, no currency symbol")]
    pub amount: Option<f64>,
    #[schemars(description = "ISO currency code or symbol")]
    pub currency: Option<String>,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub next_billing_date: Option<String>,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub start_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedOrder {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: i64,
    #[schemars(description = "FK: id of an ExtractedOrganisation that is the seller/merchant")]
    pub organisation_id: Option<i64>,
    pub order_reference: Option<String>,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub order_date: Option<String>,
    #[schemars(
        description = "One of: placed, confirmed, shipped, out_for_delivery, delivered, cancelled, returned, refunded, unknown"
    )]
    pub status: Option<String>,
    #[schemars(description = "Numeric amount as a number — strip currency symbols")]
    pub total_amount: Option<f64>,
    #[schemars(description = "ISO currency code or symbol")]
    pub currency: Option<String>,
    #[schemars(description = "List of product/item names or descriptions from the email")]
    pub items: Option<Vec<String>>,
    pub tracking_number: Option<String>,
    #[schemars(description = "FK: id of an ExtractedTransaction that paid for this order")]
    pub transaction_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedEvent {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    #[schemars(description = "Raw date/time string exactly as it appears in the email")]
    pub event_date: Option<String>,
    #[schemars(description = "FK: id of an ExtractedLocation for this event")]
    pub location_id: Option<i64>,
    /// Email addresses of attendees (resolved to person IDs server-side after extraction).
    pub attendees: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Top-level tool payload
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    description = "All entities extracted from the email. Include only entities actually present; omit empty lists."
)]
pub struct ExtractedEntitiesParams {
    #[schemars(
        description = "Physical addresses or places mentioned (FK targets for other entities). Omit if none found."
    )]
    pub locations: Option<Vec<ExtractedLocation>>,
    #[schemars(
        description = "Companies, institutions, banks, vendors, service providers — any organisation. Omit if none found."
    )]
    pub organisations: Option<Vec<ExtractedOrganisation>>,
    #[schemars(
        description = "Named individuals — sender, recipient, or any person mentioned. Omit if none found."
    )]
    pub persons: Option<Vec<ExtractedPerson>>,
    #[schemars(description = "Bills, invoices, receipts, or statements. Omit if none found.")]
    pub bills: Option<Vec<ExtractedBill>>,
    #[schemars(description = "Confirmed or completed payment transactions. Omit if none found.")]
    pub transactions: Option<Vec<ExtractedTransaction>>,
    #[schemars(
        description = "Recurring subscriptions to services — only create if clearly a new subscription, not just a renewal bill. Omit if none found."
    )]
    pub subscriptions: Option<Vec<ExtractedSubscription>>,
    #[schemars(description = "Product or e-commerce orders. Omit if none found.")]
    pub orders: Option<Vec<ExtractedOrder>>,
    #[schemars(description = "Meetings, appointments, or scheduled events. Omit if none found.")]
    pub events: Option<Vec<ExtractedEvent>>,
}

// ---------------------------------------------------------------------------
// Confirmation tool payload
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    description = "Confirm whether the parsed entity values shown to you are correct. If not, call submit_entities again with corrections."
)]
pub struct ConfirmEntitiesParams {
    #[schemars(
        description = "true if the parsed values are correct and you accept them; false to revise"
    )]
    pub confirmed: bool,
    #[schemars(description = "Optional note explaining corrections (only when confirmed=false)")]
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsed value types (agent-side only, not sent to LLM)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ParsedValue {
    Int(i64),
    Float(f64),
    Date(chrono::NaiveDate),
    Text(String),
}

impl std::fmt::Display for ParsedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsedValue::Int(v) => write!(f, "{} (int)", v),
            ParsedValue::Float(v) => write!(f, "{:.4} (float)", v),
            ParsedValue::Date(d) => write!(f, "{} (date)", d.format("%Y-%m-%d")),
            ParsedValue::Text(s) => write!(f, "{}", s),
        }
    }
}

pub fn parse_value(raw: &str) -> ParsedValue {
    let trimmed = raw.trim();
    if let Ok(v) = trimmed.parse::<i64>() {
        return ParsedValue::Int(v);
    }
    // Strip commas before float parse (e.g. "1,299.50")
    let no_comma = trimmed.replace(',', "");
    if let Ok(v) = no_comma.parse::<f64>() {
        return ParsedValue::Float(v);
    }
    if let Ok(dt) = dateparser::parse(trimmed) {
        return ParsedValue::Date(dt.date_naive());
    }
    ParsedValue::Text(trimmed.to_string())
}
