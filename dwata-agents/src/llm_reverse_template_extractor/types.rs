use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReverseTemplateType {
    Bill,
    Transaction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReverseTemplateField {
    TotalAmount,
    Currency,
    IssuedDate,
    DueDate,
    BillingPeriodStart,
    BillingPeriodEnd,
    DocumentReference,
    ServiceIdentifier,
    Amount,
    TransactionDate,
    Vendor,
    TransactionReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReverseVariableTranslation {
    pub placeholder_name: String,
    pub target_field: ReverseTemplateField,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    description = "A reversed Jinja2 template for one financial sample. Placeholders must use canonical field names."
)]
pub struct ReverseTemplateParams {
    #[schemars(
        description = "Jinja2 template including `Subject: ...` and `---` body separator. Placeholders must be canonical fields for the template type."
    )]
    pub template_body: String,
}
