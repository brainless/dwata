use nocodo_llm_sdk::models::ollama::QWEN_3_5_2B_ID;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

mod oauth_defaults {
    include!(concat!(env!("OUT_DIR"), "/dwata_oauth_defaults.rs"));
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub deploy: Option<DeployConfig>,
    pub database: Option<DatabaseConfig>,
    pub server: Option<ServerConfig>,
    pub gui: Option<GuiConfig>,
    pub jwt: Option<JwtConfig>,
    pub cors: Option<CorsConfig>,
    pub google_oauth: Option<GoogleOAuthConfig>,
    pub email_downloads: Option<EmailDownloadsConfig>,
    pub search: Option<SearchConfig>,
    pub ai_provider_api_keys: Option<AiProviderApiKeysConfig>,
    pub selected_llm: Option<SelectedLlmConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            deploy: None,
            database: None,
            server: Some(ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            }),
            gui: Some(GuiConfig { port: 3030 }),
            jwt: Some(JwtConfig {
                secret: "change-me-in-production".to_string(),
                expiration_hours: 24,
            }),
            cors: Some(CorsConfig {
                allowed_origins: vec!["http://localhost:3030".to_string()],
            }),
            google_oauth: Some(GoogleOAuthConfig::default()),
            email_downloads: Some(EmailDownloadsConfig::default()),
            search: Some(SearchConfig::default()),
            ai_provider_api_keys: None,
            selected_llm: Some(SelectedLlmConfig::default()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeployConfig {
    pub server_ip: String,
    pub ssh_user: String,
    pub domain_name: String,
    pub letsencrypt_email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GuiConfig {
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmailDownloadsConfig {
    pub auto_start: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchConfig {
    pub index_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AiProviderApiKeysConfig {
    pub openai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SelectedLlmConfig {
    pub provider: String,
    pub model: String,
}

impl Default for SelectedLlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: QWEN_3_5_2B_ID.to_string(),
        }
    }
}

impl Default for EmailDownloadsConfig {
    fn default() -> Self {
        Self { auto_start: false }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { index_path: None }
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

impl GoogleOAuthConfig {
    pub fn apply_compiled_defaults(&mut self) {
        if self.client_id.trim().is_empty() && !oauth_defaults::DEFAULT_GOOGLE_CLIENT_ID.is_empty()
        {
            self.client_id = oauth_defaults::DEFAULT_GOOGLE_CLIENT_ID.to_string();
        }

        if self.client_secret.is_none() {
            if let Some(secret) = oauth_defaults::DEFAULT_GOOGLE_CLIENT_SECRET {
                if !secret.is_empty() {
                    self.client_secret = Some(secret.to_string());
                }
            }
        }
    }
}

impl ApiConfig {
    pub fn load() -> Result<(Self, PathBuf), String> {
        if let Some(config_path) = Self::find_config_file() {
            let contents = fs::read_to_string(&config_path).map_err(|e| {
                format!(
                    "Failed to read config file at {}: {}",
                    config_path.display(),
                    e
                )
            })?;

            let config: ApiConfig = toml::from_str(&contents)
                .map_err(|e| format!("Failed to parse TOML config: {}", e))?;

            return Ok((config, config_path));
        }

        // No config file found anywhere — create a default one at the OS config dir.
        let config_path = dirs::config_dir()
            .map(|d| d.join("dwata").join("config.toml"))
            .ok_or_else(|| "Cannot determine OS config directory".to_string())?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create config directory {}: {}",
                    parent.display(),
                    e
                )
            })?;
        }

        let default_config = ApiConfig::default();
        let toml_string = toml::to_string(&default_config)
            .map_err(|e| format!("Failed to serialize default config: {}", e))?;

        fs::write(&config_path, &toml_string).map_err(|e| {
            format!(
                "Failed to write default config to {}: {}",
                config_path.display(),
                e
            )
        })?;

        Ok((default_config, config_path))
    }

    fn exe_dir() -> Option<PathBuf> {
        std::env::current_exe()
            .ok()?
            .parent()
            .map(|p| p.to_path_buf())
    }

    fn find_config_file() -> Option<PathBuf> {
        let mut candidates = vec![
            PathBuf::from("config.toml"),
            PathBuf::from("../config.toml"),
        ];
        if let Some(config_dir) = dirs::config_dir() {
            candidates.push(config_dir.join("dwata").join("config.toml"));
        }
        if let Some(dir) = Self::exe_dir() {
            candidates.push(dir.join("../../config.toml"));
            candidates.push(dir.join("../config.toml"));
            candidates.push(dir.join("config.toml"));
        }
        candidates.into_iter().find(|p| p.exists())
    }
}

pub fn get_config_path() -> PathBuf {
    ApiConfig::find_config_file().unwrap_or_else(|| {
        dirs::config_dir()
            .map(|d| d.join("dwata").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    })
}
