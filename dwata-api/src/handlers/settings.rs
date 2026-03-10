use crate::config::ApiConfig;
use actix_web::{web, HttpResponse, Result};
use shared_types::{
    AiProviderApiKeyConfig, OAuthClientAppConfig, SettingsResponse, UpdateAiProviderApiKeysRequest,
    UpdateOAuthClientAppsRequest,
};
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct SettingsAppState {
    pub config: Arc<std::sync::RwLock<ApiConfig>>,
}

fn mask_api_key(key: &Option<String>) -> Option<String> {
    key.as_ref().map(|k| {
        if k.len() <= 6 {
            k.clone()
        } else {
            let masked = format!("{}{}", &k[..6], "*".repeat(k.len() - 6));
            if masked.len() > 40 {
                format!("{}...", &masked[..37])
            } else {
                masked
            }
        }
    })
}

pub async fn get_settings(data: web::Data<SettingsAppState>) -> Result<HttpResponse> {
    let config = data.config.read().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to acquire config read lock: {}",
            e
        ))
    })?;

    let ai_provider_api_keys = if let Some(ref keys) = config.ai_provider_api_keys {
        vec![
            AiProviderApiKeyConfig {
                name: "openai".to_string(),
                key: mask_api_key(&keys.openai_api_key),
                is_configured: keys.openai_api_key.is_some(),
            },
            AiProviderApiKeyConfig {
                name: "gemini".to_string(),
                key: mask_api_key(&keys.gemini_api_key),
                is_configured: keys.gemini_api_key.is_some(),
            },
        ]
    } else {
        vec![]
    };

    // Check if using default dwata OAuth app or custom config
    let oauth_client_apps = if let Some(ref oauth) = config.google_oauth {
        // Check if this is a custom config (non-empty from config file)
        // The system applies compiled defaults in main.rs via apply_compiled_defaults()
        // We show masked values regardless of source (default or custom)
        vec![OAuthClientAppConfig {
            provider: "google".to_string(),
            client_id: mask_api_key(&Some(oauth.client_id.clone())),
            client_secret: mask_api_key(&oauth.client_secret),
            is_configured: !oauth.client_id.is_empty(),
        }]
    } else {
        vec![OAuthClientAppConfig {
            provider: "google".to_string(),
            client_id: None,
            client_secret: None,
            is_configured: false,
        }]
    };

    let config_path = crate::config::get_config_path();
    let config_path = if config_path.is_absolute() {
        config_path
    } else {
        std::fs::canonicalize(&config_path).unwrap_or_else(|_| {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(&config_path)
        })
    };
    let response = SettingsResponse {
        config_file_path: config_path.to_string_lossy().to_string(),
        ai_provider_api_keys,
        oauth_client_apps,
        projects_default_path: None,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_ai_provider_api_keys(
    data: web::Data<SettingsAppState>,
    request: web::Json<UpdateAiProviderApiKeysRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let req = request.into_inner();

    let mut config = data.config.write().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to acquire config write lock: {}",
            e
        ))
    })?;

    if let Some(ref mut keys) = config.ai_provider_api_keys {
        if let Some(openai_key) = req.openai_api_key {
            keys.openai_api_key = Some(openai_key);
        }
        if let Some(gemini_key) = req.gemini_api_key {
            keys.gemini_api_key = Some(gemini_key);
        }
    } else {
        config.ai_provider_api_keys = Some(crate::config::AiProviderApiKeysConfig {
            openai_api_key: req.openai_api_key,
            gemini_api_key: req.gemini_api_key,
        });
    }

    let config_clone = config.clone();

    let toml_string = toml::to_string(&config_clone).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to serialize config: {}", e))
    })?;

    let config_path = crate::config::get_config_path();

    std::fs::write(&config_path, toml_string).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to write config file: {}", e))
    })?;

    info!("Updated AI provider API keys in settings");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "path": config_path.to_string_lossy()
    })))
}

pub async fn update_oauth_client_apps(
    data: web::Data<SettingsAppState>,
    request: web::Json<UpdateOAuthClientAppsRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let req = request.into_inner();

    let mut config = data.config.write().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to acquire config write lock: {}",
            e
        ))
    })?;

    if let Some(ref mut oauth) = config.google_oauth {
        if let Some(client_id) = req.google_client_id {
            oauth.client_id = client_id;
        }
        if let Some(client_secret) = req.google_client_secret {
            oauth.client_secret = Some(client_secret);
        }
    } else {
        config.google_oauth = Some(crate::config::GoogleOAuthConfig {
            client_id: req.google_client_id.unwrap_or_default(),
            client_secret: req.google_client_secret,
        });
    }

    let config_clone = config.clone();

    let toml_string = toml::to_string(&config_clone).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to serialize config: {}", e))
    })?;

    let config_path = crate::config::get_config_path();

    std::fs::write(&config_path, toml_string).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to write config file: {}", e))
    })?;

    info!("Updated OAuth client apps in settings");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "path": config_path.to_string_lossy()
    })))
}
