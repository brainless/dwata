use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Primary type of a financial document.
///
/// Determines which downstream extractors to run and what data to expect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentType {
    /// Inward bill: you owe money (utility, subscription, rent, etc.)
    Bill,
    /// Outward invoice: you are owed money (future use)
    Invoice,
    /// Confirms a completed outgoing payment (bank or payment processor side)
    PaymentConfirmation,
    /// Post-payment receipt from the merchant
    Receipt,
    /// Bulk statement with multiple transactions (handled by a different pipeline)
    BankStatement,
    /// Tax-related document
    TaxDocument,
    /// No applicable financial document type from the supported set.
    #[serde(
        alias = "none",
        alias = "none_applicable",
        alias = "none-applicable",
        alias = "none applicable"
    )]
    Unknown,
}

/// Result of the document labeler agent.
///
/// Drives which downstream extractors are run on this template.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    description = "Classify the financial document type and determine what structured data it contains."
)]
pub struct LabelDocumentParams {
    /// The primary type of this financial document. Choose the single best match:
    /// - "bill": you owe money — has amount due, due date, billing period
    /// - "invoice": you are owed money
    /// - "payment-confirmation": a bank/processor confirms payment was made
    /// - "receipt": merchant-side post-payment confirmation
    /// - "bank-statement": bulk statement (multiple transactions)
    /// - "tax-document": tax-related document
    #[schemars(
        description = "Primary document type. One of: bill, invoice, payment-confirmation, receipt, bank-statement, tax-document"
    )]
    pub doc_type: DocumentType,

    /// True if the document contains a payable/due amount with a due date or billing period.
    /// Signals: "amount due", "due by", "billing period", "pay by", "your bill".
    #[schemars(
        description = "True if document has an amount due, due date, or billing period (needs bill extraction)"
    )]
    pub has_bill: bool,

    /// True if the document confirms a completed payment or debit.
    /// Signals: "payment received", "you paid", "debited", "transaction successful", "amount charged".
    #[schemars(
        description = "True if document confirms a completed payment or debit (needs transaction extraction)"
    )]
    pub has_transaction: bool,
}
