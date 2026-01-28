use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum CredentialType {
    Imap,
    Smtp,
    OAuth,
    ApiKey,
    Database,
    Custom,
}

impl CredentialType {
    pub fn service_name(&self) -> String {
        format!("dwata:{}", self.as_str())
    }

    pub fn as_str(&self) -> &str {
        match self {
            CredentialType::Imap => "imap",
            CredentialType::Smtp => "smtp",
            CredentialType::OAuth => "oauth",
            CredentialType::ApiKey => "apikey",
            CredentialType::Database => "database",
            CredentialType::Custom => "custom",
        }
    }
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCredentialRequest {
    pub credential_type: CredentialType,
    pub identifier: String,
    pub username: String,
    pub password: String,
    pub service_name: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
    pub notes: Option<String>,
    pub extra_metadata: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCredentialRequest {
    pub username: Option<String>,
    pub password: Option<String>,
    pub service_name: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
    pub notes: Option<String>,
    pub extra_metadata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CredentialMetadata {
    pub id: String,
    pub credential_type: CredentialType,
    pub identifier: String,
    pub username: String,
    pub service_name: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_accessed_at: Option<i64>,
    pub is_active: bool,
    pub extra_metadata: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PasswordResponse {
    pub password: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CredentialListResponse {
    pub credentials: Vec<CredentialMetadata>,
}
