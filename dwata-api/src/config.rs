use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub api_keys: Option<ApiKeysConfig>,
    pub cors: Option<CorsConfig>,
    pub server: Option<ServerConfig>,
    pub google_oauth: Option<GoogleOAuthConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            api_keys: None,
            cors: Some(CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            }),
            server: Some(ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            }),
            google_oauth: Some(GoogleOAuthConfig::default()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiKeysConfig {
    pub gemini_api_key: Option<String>,
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
    pub redirect_uri: String,
}

impl Default for GoogleOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: "".to_string(),
            redirect_uri: "http://localhost:8080/api/oauth/google/callback".to_string(),
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

[cors]
allowed_origins = ["http://localhost:3030"]

[server]
host = "127.0.0.1"
port = 8080

[google_oauth]
# Google Cloud Console OAuth2 client ID for Gmail
# client_id = "YOUR_CLIENT_ID.apps.googleusercontent.com"
# redirect_uri = "http://localhost:8080/api/oauth/google/callback"
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
