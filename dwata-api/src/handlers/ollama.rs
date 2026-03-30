use actix_web::{web, HttpResponse, Result};
use nocodo_llm_sdk::models::ollama::QWEN_3_5_2B_ID;
use nocodo_llm_sdk::ollama::types as sdk_types;
use nocodo_llm_sdk::ollama::OllamaClient;
use shared_types::{
    ErrorResponse, OllamaModelDetails, OllamaModelInfo, OllamaModelsResponse,
    OllamaPullModelRequest, OllamaPullModelResponse, OllamaStatusResponse,
};

const ALLOWED_PULL_MODELS: &[&str] = &[QWEN_3_5_2B_ID];

fn map_model_details(details: sdk_types::OllamaModelDetails) -> OllamaModelDetails {
    OllamaModelDetails {
        parent_model: details.parent_model,
        format: details.format,
        family: details.family,
        families: details.families,
        parameter_size: details.parameter_size,
        quantization_level: details.quantization_level,
    }
}

fn map_model_info(model: sdk_types::OllamaModelInfo) -> OllamaModelInfo {
    OllamaModelInfo {
        name: model.name,
        model: model.model,
        modified_at: model.modified_at,
        size: model.size,
        digest: model.digest,
        details: map_model_details(model.details),
    }
}

fn map_pull_status(status: sdk_types::OllamaPullStatus) -> OllamaPullModelResponse {
    OllamaPullModelResponse {
        status: status.status,
        digest: status.digest,
        total: status.total,
        completed: status.completed,
    }
}

pub async fn ollama_status() -> Result<HttpResponse> {
    let client = OllamaClient::new().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to initialize Ollama client: {}",
            e
        ))
    })?;

    let response = OllamaStatusResponse {
        running: client.is_running().await,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn ollama_list_models() -> Result<HttpResponse> {
    let client = OllamaClient::new().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to initialize Ollama client: {}",
            e
        ))
    })?;

    let models = client.list_models().await.map_err(|e| {
        actix_web::error::ErrorBadGateway(format!("Failed to list Ollama models: {}", e))
    })?;

    let response = OllamaModelsResponse {
        models: models.into_iter().map(map_model_info).collect(),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn ollama_pull_model(request: web::Json<OllamaPullModelRequest>) -> Result<HttpResponse> {
    let req = request.into_inner();

    if !ALLOWED_PULL_MODELS.contains(&req.model.as_str()) {
        let response = ErrorResponse {
            error: format!(
                "Model not allowed. Allowed models: {}",
                ALLOWED_PULL_MODELS.join(", ")
            ),
        };
        return Ok(HttpResponse::BadRequest().json(response));
    }

    let client = OllamaClient::new().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to initialize Ollama client: {}",
            e
        ))
    })?;

    let mut last_status: Option<sdk_types::OllamaPullStatus> = None;
    client
        .pull_model(req.model, |status| {
            last_status = Some(status);
        })
        .await
        .map_err(|e| actix_web::error::ErrorBadGateway(format!("Failed to pull model: {}", e)))?;

    let response = last_status
        .map(map_pull_status)
        .unwrap_or(OllamaPullModelResponse {
            status: "completed".to_string(),
            digest: None,
            total: None,
            completed: None,
        });

    Ok(HttpResponse::Ok().json(response))
}
