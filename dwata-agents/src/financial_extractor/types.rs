use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestPatternParams {
    pub regex_pattern: String,
    pub amount_group: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_group: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SavePatternParams {
    pub name: String,
    pub regex_pattern: String,
    pub document_type: String,
    pub status: String,
    pub amount_group: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_group: Option<usize>,
}
