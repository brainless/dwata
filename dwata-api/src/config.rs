use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub api_keys: Option<ApiKeysConfig>,
    pub database: Option<DatabaseConfig>,
    pub cors: Option<CorsConfig>,
    pub server: Option<ServerConfig>,
    pub google_oauth: Option<GoogleOAuthConfig>,
    pub downloads: Option<DownloadsConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            api_keys: None,
            database: None,
            cors: Some(CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            }),
            server: Some(ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            }),
            google_oauth: Some(GoogleOAuthConfig::default()),
            downloads: Some(DownloadsConfig::default()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiKeysConfig {
    pub gemini_api_key: Option<String>,
    pub claude_api_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DownloadsConfig {
    pub auto_start: bool,
}

impl Default for DownloadsConfig {
    fn default() -> Self {
        Self { auto_start: false }
    }
}

impl Default for GoogleOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: "".to_string(),
            client_secret: None,
        }
    }
}

impl ApiConfig {
    pub fn load() -> Result<(Self, PathBuf), ConfigError> {
        let config_path = get_config_path();

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ConfigError::Message(format!("Failed to create config directory: {e}"))
            })?;
        }

        // Create default config file if it doesn't exist
        if !config_path.exists() {
            let default_config = r#"
[api_keys]
# gemini_api_key = "your-gemini-key"
# claude_api_key = "your-claude-key"

[database]
# path = "/absolute/path/to/db.sqlite"

[cors]
allowed_origins = ["http://localhost:3030"]

[server]
host = "127.0.0.1"
port = 8080

[google_oauth]
# Google Cloud Console OAuth2 client ID for Gmail
# client_id = "YOUR_CLIENT_ID.apps.googleusercontent.com"
# client_secret = "YOUR_CLIENT_SECRET" # Optional, for Desktop apps
# Redirect URI is automatically constructed from server host and port

[downloads]
# When false, the API will not auto-start download jobs on startup.
auto_start = false
"#;
            std::fs::write(&config_path, default_config).map_err(|e| {
                ConfigError::Message(format!("Failed to write default config: {e}"))
            })?;
        }

        let builder = Config::builder()
            .add_source(File::from(config_path.clone()))
            .build()?;

        let config: ApiConfig = builder.try_deserialize()?;

        Ok((config, config_path))
    }
}

pub fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("dwata").join("api.toml")
    } else {
        PathBuf::from("api.toml")
    }
}
