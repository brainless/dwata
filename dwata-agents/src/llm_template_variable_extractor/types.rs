use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateVariableType {
    Bill,
    Transaction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TemplateVariable {
    #[schemars(
        description = "The canonical variable name (e.g., total_amount, currency, vendor_name)"
    )]
    pub variable_name: String,
    #[schemars(description = "The actual value extracted from the email sample")]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    description = "List of template variables extracted from the email sample with their values."
)]
pub struct TemplateVariableParams {
    #[schemars(description = "List of variables found in the email with their values")]
    pub variables: Vec<TemplateVariable>,
}
