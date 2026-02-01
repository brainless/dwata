use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CredentialType {
    Imap,
    Smtp,
    OAuth,
    ApiKey,
    Database,
    LocalFile,
    Custom,
}

/// Authentication method for IMAP
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ImapAuthMethod {
    Plain,
    OAuth2,
    Xoauth2,
}

/// IMAP-specific account settings
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ImapAccountSettings {
    /// IMAP server host (e.g., "imap.gmail.com")
    pub host: String,
    /// IMAP server port (typically 993 for SSL, 143 for non-SSL)
    pub port: i32,
    /// Use TLS/SSL connection
    pub use_tls: bool,
    /// Authentication method
    #[serde(default = "default_auth_method")]
    pub auth_method: ImapAuthMethod,
    /// Default mailbox/folder to monitor (default: "INBOX")
    #[serde(default = "default_mailbox")]
    pub default_mailbox: String,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connection_timeout_secs: u32,
    /// Whether to validate SSL certificates (should be true in production)
    #[serde(default = "default_validate_certs")]
    pub validate_certs: bool,
}

fn default_auth_method() -> ImapAuthMethod {
    ImapAuthMethod::Plain
}

fn default_mailbox() -> String {
    "INBOX".to_string()
}

fn default_timeout() -> u32 {
    30
}

fn default_validate_certs() -> bool {
    true
}

impl Default for ImapAccountSettings {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 993,
            use_tls: true,
            auth_method: ImapAuthMethod::Plain,
            default_mailbox: "INBOX".to_string(),
            connection_timeout_secs: 30,
            validate_certs: true,
        }
    }
}

/// SMTP-specific account settings
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SmtpAccountSettings {
    /// SMTP server host (e.g., "smtp.gmail.com")
    pub host: String,
    /// SMTP server port (typically 587 for STARTTLS, 465 for SSL)
    pub port: i32,
    /// Use TLS/SSL connection
    pub use_tls: bool,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connection_timeout_secs: u32,
}

/// API Key service settings
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ApiKeySettings {
    /// Base URL for the API (e.g., "https://api.stripe.com")
    pub base_url: String,
    /// API version (if applicable)
    pub api_version: Option<String>,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u32,
}

/// Local file path settings
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct LocalFileSettings {
    /// Absolute path to the file or directory
    pub file_path: String,
    /// Optional description of what this file contains
    pub description: Option<String>,
    /// File type hint (e.g., "linkedin-archive", "email-export")
    pub file_type: Option<String>,
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
            CredentialType::LocalFile => "localfile",
            CredentialType::Custom => "custom",
        }
    }

    /// Check if this credential type requires keychain storage
    pub fn requires_keychain(&self) -> bool {
        !matches!(self, CredentialType::LocalFile)
    }
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCredentialRequest {
    pub credential_type: CredentialType,
    pub identifier: String,
    pub username: String,
    /// Password is optional for credential types that don't require keychain storage (e.g., LocalFile)
    pub password: Option<String>,
    pub service_name: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
    pub notes: Option<String>,
    pub extra_metadata: Option<String>,
}

/// Type-safe request for creating IMAP credentials
#[derive(Debug, Deserialize, TS)]
pub struct CreateImapCredentialRequest {
    pub identifier: String,
    pub username: String,
    pub password: String,
    pub settings: ImapAccountSettings,
    pub notes: Option<String>,
}

impl CreateImapCredentialRequest {
    /// Convert to generic CreateCredentialRequest
    pub fn into_generic(self) -> Result<CreateCredentialRequest, serde_json::Error> {
        Ok(CreateCredentialRequest {
            credential_type: CredentialType::Imap,
            identifier: self.identifier,
            username: self.username,
            password: Some(self.password),
            service_name: Some(self.settings.host.clone()),
            port: Some(self.settings.port),
            use_tls: Some(self.settings.use_tls),
            notes: self.notes,
            extra_metadata: Some(self.settings.to_json_string()?),
        })
    }
}

/// Type-safe request for creating local file credentials
#[derive(Debug, Deserialize, TS)]
pub struct CreateLocalFileCredentialRequest {
    pub identifier: String,
    pub settings: LocalFileSettings,
    pub notes: Option<String>,
}

impl CreateLocalFileCredentialRequest {
    /// Convert to generic CreateCredentialRequest
    pub fn into_generic(self) -> Result<CreateCredentialRequest, serde_json::Error> {
        Ok(CreateCredentialRequest {
            credential_type: CredentialType::LocalFile,
            identifier: self.identifier,
            username: "local".to_string(),
            password: None,
            service_name: None,
            port: None,
            use_tls: None,
            notes: self.notes,
            extra_metadata: Some(self.settings.to_json_string()?),
        })
    }
}

#[derive(Debug, Deserialize, TS)]
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
pub struct CredentialMetadata {
    pub id: i64,
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
pub struct PasswordResponse {
    pub password: String,
}

#[derive(Debug, Serialize, TS)]
pub struct CredentialListResponse {
    pub credentials: Vec<CredentialMetadata>,
}

/// Extended credential metadata with parsed IMAP settings
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ImapCredentialMetadata {
    pub id: i64,
    pub identifier: String,
    pub username: String,
    pub settings: ImapAccountSettings,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_accessed_at: Option<i64>,
    pub is_active: bool,
}

impl ImapCredentialMetadata {
    /// Create from generic CredentialMetadata
    pub fn from_generic(metadata: CredentialMetadata) -> Result<Self, String> {
        let settings = metadata.parse_imap_settings()?;
        Ok(Self {
            id: metadata.id,
            identifier: metadata.identifier,
            username: metadata.username,
            settings,
            notes: metadata.notes,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
            last_accessed_at: metadata.last_accessed_at,
            is_active: metadata.is_active,
        })
    }
}

// Helper functions for working with service-specific settings

impl ImapAccountSettings {
    /// Serialize to JSON string for storage in extra_metadata
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string stored in extra_metadata
    pub fn from_json_string(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl SmtpAccountSettings {
    /// Serialize to JSON string for storage in extra_metadata
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string stored in extra_metadata
    pub fn from_json_string(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl ApiKeySettings {
    /// Serialize to JSON string for storage in extra_metadata
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string stored in extra_metadata
    pub fn from_json_string(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl LocalFileSettings {
    /// Serialize to JSON string for storage in extra_metadata
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string stored in extra_metadata
    pub fn from_json_string(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl CredentialMetadata {
    /// Parse IMAP settings from extra_metadata
    pub fn parse_imap_settings(&self) -> Result<ImapAccountSettings, String> {
        match &self.extra_metadata {
            Some(json) => ImapAccountSettings::from_json_string(json)
                .map_err(|e| format!("Failed to parse IMAP settings: {}", e)),
            None => Ok(ImapAccountSettings::default()),
        }
    }

    /// Parse SMTP settings from extra_metadata
    pub fn parse_smtp_settings(&self) -> Result<SmtpAccountSettings, String> {
        match &self.extra_metadata {
            Some(json) => SmtpAccountSettings::from_json_string(json)
                .map_err(|e| format!("Failed to parse SMTP settings: {}", e)),
            None => Err("No SMTP settings found".to_string()),
        }
    }

    /// Parse API key settings from extra_metadata
    pub fn parse_api_key_settings(&self) -> Result<ApiKeySettings, String> {
        match &self.extra_metadata {
            Some(json) => ApiKeySettings::from_json_string(json)
                .map_err(|e| format!("Failed to parse API key settings: {}", e)),
            None => Err("No API key settings found".to_string()),
        }
    }

    /// Parse local file settings from extra_metadata
    pub fn parse_local_file_settings(&self) -> Result<LocalFileSettings, String> {
        match &self.extra_metadata {
            Some(json) => LocalFileSettings::from_json_string(json)
                .map_err(|e| format!("Failed to parse local file settings: {}", e)),
            None => Err("No local file settings found".to_string()),
        }
    }
}
