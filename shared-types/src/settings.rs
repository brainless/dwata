use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Configuration for an AI provider API key
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct AiProviderApiKeyConfig {
    pub name: String,
    pub key: Option<String>,
    pub is_configured: bool,
}

/// Configuration for an OAuth client app
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct OAuthClientAppConfig {
    pub provider: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub is_configured: bool,
}

/// Response for settings endpoint
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SettingsResponse {
    pub config_file_path: String,
    pub ai_provider_api_keys: Vec<AiProviderApiKeyConfig>,
    pub oauth_client_apps: Vec<OAuthClientAppConfig>,
    pub projects_default_path: Option<String>,
}

/// Request to update AI provider API keys
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct UpdateAiProviderApiKeysRequest {
    pub gemini_api_key: Option<String>,
    pub claude_api_key: Option<String>,
}

/// Request to update OAuth client apps
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct UpdateOAuthClientAppsRequest {
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
}
