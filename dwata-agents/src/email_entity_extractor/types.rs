use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Per-entity extraction structs (exposed to LLM via tool schema)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedLocation {
    #[schemars(
        description = "Unique positive integer id you assign — other entities reference this"
    )]
    pub id: u32,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    #[schemars(description = "ISO 3166-1 alpha-2 or alpha-3 country code if identifiable")]
    pub country_code: Option<String>,
    pub postal_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedCompany {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: u32,
    pub name: String,
    pub industry: Option<String>,
    pub website: Option<String>,
    #[schemars(description = "FK: id of an ExtractedLocation describing this company's address")]
    pub location_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedContact {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: u32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    #[schemars(description = "FK: id of an ExtractedCompany this person belongs to")]
    pub company_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedVendor {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: u32,
    pub vendor_name: String,
    #[schemars(
        description = "One of: self_user, self_business, financial_instrument, merchant, employer, bank, individual, platform, unknown"
    )]
    pub vendor_type: String,
    pub vendor_external_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedBill {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: u32,
    #[schemars(
        description = "One of: invoice, bill, bank_statement, receipt, tax_document, payment_confirmation"
    )]
    pub document_type: Option<String>,
    #[schemars(
        description = "Numeric amount string only — strip currency symbols (e.g. '299.00')"
    )]
    pub total_amount: Option<String>,
    #[schemars(description = "ISO currency code or symbol (e.g. INR, USD, $)")]
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
    #[schemars(description = "FK: id of an ExtractedVendor that issued this bill")]
    pub issuer_vendor_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedTransaction {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: u32,
    #[schemars(
        description = "Numeric amount string only — strip currency symbols (e.g. '1299.50')"
    )]
    pub amount: String,
    #[schemars(description = "ISO currency code or symbol (e.g. INR, USD, $)")]
    pub currency: String,
    #[schemars(description = "Raw date string exactly as it appears in the email")]
    pub transaction_date: Option<String>,
    pub transaction_reference: Option<String>,
    #[schemars(description = "FK: id of an ExtractedVendor who paid")]
    pub payer_vendor_id: Option<u32>,
    #[schemars(description = "FK: id of an ExtractedVendor who received payment")]
    pub payee_vendor_id: Option<u32>,
    #[schemars(description = "FK: id of an ExtractedBill this transaction settles")]
    pub bill_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedEvent {
    #[schemars(description = "Unique positive integer id you assign")]
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    #[schemars(description = "Raw date/time string exactly as it appears in the email")]
    pub event_date: Option<String>,
    #[schemars(description = "FK: id of an ExtractedLocation for this event")]
    pub location_id: Option<u32>,
    pub attendees: Vec<String>,
}

// ---------------------------------------------------------------------------
// Top-level tool payload
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    description = "All entities extracted from the email. Include only entities actually present in the email; omit empty lists."
)]
pub struct ExtractedEntitiesParams {
    #[schemars(
        description = "Physical addresses, cities, or countries found in the email (used as FK targets by other entities)"
    )]
    pub locations: Vec<ExtractedLocation>,
    #[schemars(description = "Companies, organisations, or institutions mentioned")]
    pub companies: Vec<ExtractedCompany>,
    #[schemars(description = "Named individuals (sender, recipient, contacts)")]
    pub contacts: Vec<ExtractedContact>,
    #[schemars(description = "Vendors / merchants / banks / payment parties")]
    pub vendors: Vec<ExtractedVendor>,
    #[schemars(description = "Bills, invoices, receipts, or statements")]
    pub bills: Vec<ExtractedBill>,
    #[schemars(description = "Completed or confirmed payment transactions")]
    pub transactions: Vec<ExtractedTransaction>,
    #[schemars(description = "Meetings, appointments, or scheduled events")]
    pub events: Vec<ExtractedEvent>,
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
// Parsed entity structs (agent-side only, not sent to LLM)
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
    // Try int
    if let Ok(v) = trimmed.parse::<i64>() {
        return ParsedValue::Int(v);
    }
    // Try float (remove commas first for numbers like "1,299.50")
    let no_comma = trimmed.replace(',', "");
    if let Ok(v) = no_comma.parse::<f64>() {
        return ParsedValue::Float(v);
    }
    // Try date via dateparser
    if let Ok(dt) = dateparser::parse(trimmed) {
        return ParsedValue::Date(dt.date_naive());
    }
    ParsedValue::Text(trimmed.to_string())
}
