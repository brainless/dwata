use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fields that can be extracted from a bill or invoice document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum BillField {
    /// Total amount payable or due on the bill (numeric value only)
    TotalAmount,
    /// Currency code or symbol (e.g., "USD", "INR", "$", "₹")
    Currency,
    /// Date the bill or invoice was generated/issued by the vendor
    IssuedDate,
    /// Payment due date
    DueDate,
    /// Start of the billing or service period
    BillingPeriodStart,
    /// End of the billing or service period
    BillingPeriodEnd,
    /// Bill, invoice, or reference number/ID from the issuer
    DocumentReference,
    /// Service account identifier (phone number, account number, policy number, etc.)
    ServiceIdentifier,
}

/// What kind of service identifier is present on the bill.
///
/// Only relevant when `field` is `service-identifier`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceIdentifierKind {
    PhoneNumber,
    AccountNumber,
    PolicyNumber,
    MeterNumber,
    SubscriptionId,
    ContractId,
    Other,
}

/// A single placeholder-to-bill-field mapping.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BillVariableTranslation {
    #[schemars(
        description = "The generic placeholder name from the template, e.g. 'placeholder_1' or 'subject_1'"
    )]
    pub placeholder: String,

    /// The bill field this placeholder maps to.
    /// Use one of: total-amount, currency, issued-date, due-date, billing-period-start,
    /// billing-period-end, document-reference, service-identifier.
    /// Null if the placeholder does not map to any bill field.
    #[schemars(
        description = "The bill field this placeholder maps to. One of: total-amount, currency, issued-date, due-date, billing-period-start, billing-period-end, document-reference, service-identifier. Null if not a bill field."
    )]
    pub field: Option<BillField>,

    /// Only set when field is service-identifier. Identifies the kind of service identifier.
    /// One of: phone-number, account-number, policy-number, meter-number, subscription-id, contract-id, other.
    #[schemars(
        description = "Only set when field is service-identifier. One of: phone-number, account-number, policy-number, meter-number, subscription-id, contract-id, other."
    )]
    pub service_identifier_kind: Option<ServiceIdentifierKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Translate template placeholder names to bill/invoice field names.")]
pub struct TranslateBillVariablesParams {
    #[schemars(description = "List of translations from placeholder names to bill field names.")]
    pub translations: Vec<BillVariableTranslation>,
}

impl TranslateBillVariablesParams {
    /// Convert to a HashMap for easy lookup: placeholder → (field, service_kind).
    pub fn to_map(&self) -> HashMap<String, Option<BillField>> {
        self.translations
            .iter()
            .map(|t| (t.placeholder.clone(), t.field.clone()))
            .collect()
    }
}
