use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::database::Database;
use dwata_agents::{
    financial_extractor::FinancialExtractorAgent,
    storage::{sqlite_storage::SqliteAgentStorage, Session},
    tools::DwataToolExecutor,
};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::GEMINI_3_FLASH_ID;
use crate::financial_keywords::{DEFAULT_FINANCIAL_KEYWORDS, build_fts_query};

#[derive(Debug, Deserialize)]
pub struct GeneratePatternRequest {
    pub email_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct ProcessSenderRequest {
    pub sender_email: String,
    pub credential_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct GeneratePatternResponse {
    pub session_id: i64,
    pub status: String,
    pub pattern_id: Option<i64>,
    pub extracted_data: Vec<shared_types::FinancialTransaction>,
}

async fn run_financial_extractor(
    email_id: i64,
    db: &Database,
    config: &crate::config::ApiConfig,
) -> anyhow::Result<GeneratePatternResponse> {
    let email = crate::database::emails::get_email(
        db.async_connection.clone(),
        email_id,
    )
    .await?;

    let patterns = crate::database::financial_patterns::list_active_patterns(
        db.async_connection.clone(),
    )
    .await?;

    let api_key = config
        .ai_provider_api_keys
        .as_ref()
        .and_then(|k| k.gemini_api_key.as_ref())
        .ok_or_else(|| anyhow::anyhow!("Gemini API key not configured"))?;

    let llm_client: Arc<dyn LlmClient> = Arc::new(GeminiClient::new(api_key)?);

    let storage: Arc<dyn dwata_agents::AgentStorage> =
        Arc::new(SqliteAgentStorage::new(db.connection.clone()));

    let subject = email.subject.unwrap_or_else(|| "".to_string());
    let body_text = email.body_text.unwrap_or_else(|| "".to_string());

    let email_content = format!("{}\n\n{}", subject, body_text);
    let tool_executor = Arc::new(DwataToolExecutor::new(db.connection.clone(), email_content));

    let agent = FinancialExtractorAgent::new(
        llm_client,
        storage.clone(),
        tool_executor,
        GEMINI_3_FLASH_ID.to_string(),
        subject,
        body_text,
        patterns,
    );

    let session_id = storage
        .create_session(Session {
            id: None,
            agent_type: "financial-extractor".to_string(),
            objective: format!("Generate pattern for email {}", email_id),
            context_data: Some(
                serde_json::json!({
                    "email_id": email_id,
                })
                .to_string(),
            ),
            status: "running".to_string(),
            result: None,
        })
        .await?;

    let result = match agent.execute(session_id).await {
        Ok(result) => result,
        Err(e) => {
            let _ = storage
                .update_session(Session {
                    id: Some(session_id),
                    agent_type: "financial-extractor".to_string(),
                    objective: "".to_string(),
                    context_data: None,
                    status: "failed".to_string(),
                    result: Some(e.to_string()),
                })
                .await;
            return Err(e);
        }
    };

    let _ = storage
        .update_session(Session {
            id: Some(session_id),
            agent_type: "financial-extractor".to_string(),
            objective: "".to_string(),
            context_data: None,
            status: "completed".to_string(),
            result: Some(result.clone()),
        })
        .await;

    Ok(GeneratePatternResponse {
        session_id,
        status: "completed".to_string(),
        pattern_id: None,
        extracted_data: vec![],
    })
}

#[actix_web::post("/api/extraction/generate-pattern")]
pub async fn generate_pattern(
    req: web::Json<GeneratePatternRequest>,
    db: web::Data<Arc<Database>>,
    config: web::Data<Arc<crate::config::ApiConfig>>,
) -> ActixResult<HttpResponse> {
    match run_financial_extractor(req.email_id, db.as_ref(), config.as_ref()).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => {
            tracing::error!("Agent execution failed: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Agent execution failed",
                "details": e.to_string()
            })))
        }
    }
}

#[actix_web::post("/api/financial/patterns/process-sender")]
pub async fn process_sender(
    req: web::Json<ProcessSenderRequest>,
    db: web::Data<Arc<Database>>,
    config: web::Data<Arc<crate::config::ApiConfig>>,
) -> ActixResult<HttpResponse> {
    let fts_query = build_fts_query(DEFAULT_FINANCIAL_KEYWORDS);
    let email_id = match crate::database::emails::get_latest_email_id_for_sender_fts(
        db.async_connection.clone(),
        req.credential_id,
        &req.sender_email,
        &fts_query,
    )
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "No matching email found for sender"
            })));
        }
        Err(e) => {
            tracing::error!("Failed to lookup sender email: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to lookup sender email"
            })));
        }
    };

    match run_financial_extractor(email_id, db.as_ref(), config.as_ref()).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => {
            tracing::error!("Agent execution failed: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Agent execution failed",
                "details": e.to_string()
            })))
        }
    }
}
