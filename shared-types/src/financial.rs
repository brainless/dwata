use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Data source type for extracted transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum DataSourceType {
    Email,
    Imap,
    BankStatement,
    CreditCardStatement,
    BankFeed,
    CsvUpload,
    Manual,
    Unknown,
}

/// Financial document types that can be extracted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum FinancialDocumentType {
    Invoice,
    Bill,
    BankStatement,
    Receipt,
    TaxDocument,
    PaymentConfirmation,
}

/// Financial transaction extracted from documents
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialTransaction {
    pub id: i64,

    // Source tracking (agnostic to source type)
    pub data_source_type: DataSourceType,
    pub data_source_id: String, // ID in the source system

    // Transaction data
    pub document_type: FinancialDocumentType,
    pub description: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub transaction_date: String,

    // Additional fields
    pub category: Option<TransactionCategory>,
    // Transaction endpoints are always required in this model.
    // If unresolved, use PartyIdentity::Unknown or candidate identities.
    pub payer: TransactionParty,
    pub payee: TransactionParty,
    pub status: TransactionStatus,
    // Progression state for user-in-the-loop enrichment.
    pub enrichment_status: EnrichmentStatus,
    // Explicit queue of fields still waiting for user confirmation/correction.
    pub unresolved_items: Vec<UnresolvedField>,

    // Metadata
    pub source_file: Option<String>,
    pub extracted_at: i64,
    pub notes: Option<String>,
    pub transaction_reference: Option<String>,
}

/// Relative role of an endpoint party in a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum PartyRole {
    Payer,
    Payee,
}

/// Strongly typed transaction endpoint. Always present for both payer and payee.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TransactionParty {
    pub role: PartyRole,
    // Canonical identity state. Never null; unknown is explicit.
    pub identity: PartyIdentity,
    // Why this identity was assigned.
    pub evidence: Vec<PartyEvidence>,
    // UI/job systems can route this directly into follow-up tasks.
    pub needs_user_confirmation: bool,
}

/// Canonical identity of a party endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum PartyIdentity {
    // The user's own identity (workspace owner side of transaction).
    SelfEntity,
    // Resolved party in TransactionVendor table.
    KnownVendorId(i64),
    // Single candidate awaiting user confirmation.
    CandidateVendorId(i64),
    // Parsing captured a party role exists but could not infer identity yet.
    Unknown,
}

/// Provenance for party identity assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum PartyEvidence {
    ExplicitInDocument,
    PatternDefault,
    UserProvided,
}

/// Lifecycle status for user-in-the-loop transaction enrichment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum EnrichmentStatus {
    // Parser/LLM produced an initial record with potential unknowns.
    RawExtracted,
    // Some fields resolved, but transaction still has unresolved_items.
    PartiallyResolved,
    // User explicitly reviewed/confirmed key fields.
    UserConfirmed,
    // No unresolved_items remain.
    FullyResolved,
}

/// Explicitly tracked unresolved fields requiring user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum UnresolvedField {
    PayerIdentity,
    PayeeIdentity,
    Category,
    TransactionReference,
    TransactionDate,
    Currency,
}

/// Vendor type for transaction parties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionVendorType {
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
pub struct TransactionVendor {
    pub id: i64,
    pub vendor_type: TransactionVendorType,
    pub vendor_name: String,
    pub vendor_external_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Category for financial transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionCategory {
    Income,
    Expense,
    Investment,
    Tax,
    Utility,
    Subscription,
    Entertainment,
    Travel,
    Healthcare,
    Education,
    Other,
}

/// Status of financial transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionStatus {
    Pending,
    Paid,
    Overdue,
    Cancelled,
    Refunded,
}

/// Financial summary/overview
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialSummary {
    pub total_income: f64,
    pub total_expenses: f64,
    pub net_balance: f64,
    pub pending_bills: i32,
    pub overdue_payments: i32,
    pub currency: String,
    pub period_start: String,
    pub period_end: String,
}

/// Financial extraction source summary
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialExtractionSummary {
    pub source_count: i64,
    pub transaction_count: i64,
    pub last_extracted_at: Option<i64>,
}

/// Financial health metrics
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FinancialHealth {
    pub summary: FinancialSummary,
    pub recent_transactions: Vec<FinancialTransaction>,
    pub upcoming_bills: Vec<FinancialTransaction>,
    pub category_breakdown: Vec<CategoryBreakdown>,
}

/// Breakdown by category
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CategoryBreakdown {
    pub category: TransactionCategory,
    pub amount: f64,
    pub percentage: f64,
    pub transaction_count: i32,
}
