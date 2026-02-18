use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fields that can be extracted from a transaction confirmation document.
///
/// These are per-email variable fields — amounts, dates, references, vendor names.
/// Category is a sender-level fixed attribute determined by the labeler, not extracted here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionField {
    /// The transaction amount (numeric value only, no currency symbol)
    Amount,
    /// Currency code or symbol (e.g., "USD", "INR", "$", "₹")
    Currency,
    /// Date the transaction occurred or was processed
    TransactionDate,
    /// Merchant, company, or counterparty name
    Vendor,
    /// Transaction reference, confirmation number, or UTR
    TransactionReference,
}

/// A single placeholder-to-transaction-field mapping.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VariableTranslation {
    #[schemars(
        description = "The generic placeholder name from the template, e.g. 'placeholder_1' or 'subject_1'"
    )]
    pub placeholder: String,

    /// The transaction field this placeholder maps to.
    /// Use one of: amount, currency, transaction-date, vendor, transaction-reference.
    /// Null if the placeholder does not map to any transaction field.
    #[schemars(
        description = "The transaction field this placeholder maps to. One of: amount, currency, transaction-date, vendor, transaction-reference. Null if not a transaction field."
    )]
    pub field: Option<TransactionField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    description = "Translate generic template placeholder names to transaction field names."
)]
pub struct TranslateVariablesParams {
    #[schemars(
        description = "List of translations from generic placeholder names to transaction field names."
    )]
    pub translations: Vec<VariableTranslation>,
}

impl TranslateVariablesParams {
    /// Convert to a HashMap for easy lookup: placeholder → TransactionField.
    pub fn to_map(&self) -> HashMap<String, Option<TransactionField>> {
        self.translations
            .iter()
            .map(|t| (t.placeholder.clone(), t.field.clone()))
            .collect()
    }
}
