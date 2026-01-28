use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Configuration for an API key
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ApiKeyConfig {
    pub name: String,
    pub key: Option<String>,
    pub is_configured: bool,
}

/// Response for settings endpoint
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SettingsResponse {
    pub config_file_path: String,
    pub api_keys: Vec<ApiKeyConfig>,
    pub projects_default_path: Option<String>,
}

/// Request to update API keys
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct UpdateApiKeysRequest {
    pub gemini_api_key: Option<String>,
}
