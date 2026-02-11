use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Test a regex pattern against the email content to extract financial data")]
pub struct TestPatternParams {
    #[schemars(description = "The regex pattern to test (use Rust regex syntax with numbered groups)")]
    pub regex_pattern: String,
    #[schemars(description = "Which capture group number contains the transaction amount (required, starting from 1)")]
    pub amount_group: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which capture group contains the source/payer/sender name (optional)")]
    pub source_vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which capture group contains the destination/merchant/receiver name (optional, but at least one vendor group required)")]
    pub destination_vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which capture group contains the transaction date (optional)")]
    pub date_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which capture group contains the reference/invoice/confirmation ID (optional)")]
    pub reference_group: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Save a validated regex pattern to the database for future use")]
pub struct SavePatternParams {
    #[schemars(description = "Short descriptive name for the pattern (e.g., 'stripe_payment', 'aws_invoice')")]
    pub name: String,
    #[schemars(description = "The validated regex pattern (must be the same as successfully tested)")]
    pub regex_pattern: String,
    #[schemars(description = "Type of document: invoice, bill, receipt, payment-confirmation, bank-statement, or tax-document")]
    pub document_type: String,
    #[schemars(description = "Transaction status: paid, pending, overdue, cancelled, or refunded")]
    pub status: String,
    #[schemars(description = "Which capture group number contains the amount (same as in test_pattern)")]
    pub amount_group: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which capture group contains the source vendor (same as in test_pattern)")]
    pub source_vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which capture group contains the destination vendor (same as in test_pattern)")]
    pub destination_vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which capture group contains the date (same as in test_pattern)")]
    pub date_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which capture group contains the reference ID (same as in test_pattern)")]
    pub reference_group: Option<usize>,
}
