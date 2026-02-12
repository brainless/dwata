use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single placeholder-to-field translation entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VariableTranslation {
    #[schemars(description = "The generic placeholder name from the template, e.g. 'placeholder_1' or 'subject_1'")]
    pub placeholder: String,

    #[schemars(description = "The Jinja2 template string using financial field names, e.g. '{{ amount }}', '{{ currency }}{{ amount }}', '{{ vendor }}', '{{ transaction_date }}', '{{ category }}'. Leave empty string if this placeholder does not map to any financial field.")]
    pub field_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Translate generic template placeholder names to financial field template strings.")]
pub struct TranslateVariablesParams {
    #[schemars(description = "List of translations from generic placeholder names to financial field template strings.")]
    pub translations: Vec<VariableTranslation>,
}

impl TranslateVariablesParams {
    /// Convert the Vec-based translations into a HashMap for easy lookup.
    pub fn to_map(&self) -> HashMap<String, Option<String>> {
        self.translations
            .iter()
            .map(|t| {
                let value = if t.field_template.is_empty() {
                    None
                } else {
                    Some(t.field_template.clone())
                };
                (t.placeholder.clone(), value)
            })
            .collect()
    }
}
