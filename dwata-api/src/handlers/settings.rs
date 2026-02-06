use crate::config::ApiConfig;
use actix_web::{web, HttpResponse, Result};
use shared_types::{ApiKeyConfig, SettingsResponse, UpdateApiKeysRequest};
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

    let api_keys = if let Some(ref keys) = config.api_keys {
        let mut keys_vec = vec![ApiKeyConfig {
            name: "gemini".to_string(),
            key: mask_api_key(&keys.gemini_api_key),
            is_configured: keys.gemini_api_key.is_some(),
        }];

        if let Some(claude_key) = &keys.claude_api_key {
            keys_vec.push(ApiKeyConfig {
                name: "claude".to_string(),
                key: mask_api_key(&Some(claude_key.clone())),
                is_configured: true,
            });
        }

        keys_vec
    } else {
        vec![]
    };

    let config_path = crate::config::get_config_path();
    let response = SettingsResponse {
        config_file_path: config_path.to_string_lossy().to_string(),
        api_keys,
        projects_default_path: None,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_api_keys(
    data: web::Data<SettingsAppState>,
    request: web::Json<UpdateApiKeysRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let req = request.into_inner();

    let mut config = data.config.write().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to acquire config write lock: {}",
            e
        ))
    })?;

    if let Some(ref mut keys) = config.api_keys {
        if let Some(gemini_key) = req.gemini_api_key {
            keys.gemini_api_key = Some(gemini_key);
        }
        if let Some(claude_key) = req.claude_api_key {
            keys.claude_api_key = Some(claude_key);
        }
    } else {
        config.api_keys = Some(crate::config::ApiKeysConfig {
            gemini_api_key: req.gemini_api_key,
            claude_api_key: req.claude_api_key,
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

    info!("Updated API keys in settings");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "path": config_path.to_string_lossy()
    })))
}
