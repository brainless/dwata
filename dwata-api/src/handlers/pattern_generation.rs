use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::database::Database;
use dwata_agents::{
    financial_extractor::FinancialExtractorAgent,
    storage::{sqlite_storage::SqliteAgentStorage, Session},
    tools::DwataToolExecutor,
};
use nocodo_llm_sdk::claude::ClaudeClient;
use nocodo_llm_sdk::client::LlmClient;

#[derive(Debug, Deserialize)]
pub struct GeneratePatternRequest {
    pub email_id: i64,
}

#[derive(Debug, Serialize)]
pub struct GeneratePatternResponse {
    pub session_id: i64,
    pub status: String,
    pub pattern_id: Option<i64>,
    pub extracted_data: Vec<shared_types::FinancialTransaction>,
}

#[actix_web::post("/api/extraction/generate-pattern")]
pub async fn generate_pattern(
    req: web::Json<GeneratePatternRequest>,
    db: web::Data<Arc<Database>>,
    config: web::Data<Arc<crate::config::ApiConfig>>,
) -> ActixResult<HttpResponse> {
    let email = match crate::database::emails::get_email(
        db.async_connection.clone(),
        req.email_id,
    ).await {
        Ok(email) => email,
        Err(e) => {
            tracing::error!("Failed to get email: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get email"
            })));
        }
    };

    let patterns = match crate::database::financial_patterns::list_active_patterns(
        db.async_connection.clone(),
    ).await {
        Ok(patterns) => patterns,
        Err(e) => {
            tracing::error!("Failed to list patterns: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to list patterns"
            })));
        }
    };

    let api_key = match config.api_keys.as_ref().and_then(|k| k.claude_api_key.as_ref()) {
        Some(key) => key,
        None => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Claude API key not configured"
            })));
        }
    };

    let llm_client: Arc<dyn LlmClient> = match ClaudeClient::new(api_key) {
        Ok(client) => Arc::new(client),
        Err(e) => {
            tracing::error!("Failed to create Claude client: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create Claude client"
            })));
        }
    };

    let storage: Arc<dyn dwata_agents::AgentStorage> = Arc::new(
        SqliteAgentStorage::new(db.connection.clone())
    );

    let subject = email.subject.unwrap_or_else(|| "".to_string());
    let body_text = email.body_text.unwrap_or_else(|| "".to_string());

    let email_content = format!("{}\n\n{}", subject, body_text);
    let tool_executor = Arc::new(
        DwataToolExecutor::new(db.connection.clone(), email_content)
    );

    let agent = FinancialExtractorAgent::new(
        llm_client,
        storage.clone(),
        tool_executor,
        subject,
        body_text,
        patterns,
    );

    let session_id = match storage.create_session(Session {
        id: None,
        agent_type: "financial-extractor".to_string(),
        objective: format!("Generate pattern for email {}", req.email_id),
        context_data: Some(serde_json::json!({
            "email_id": req.email_id,
        }).to_string()),
        status: "running".to_string(),
        result: None,
    }).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to create session: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create session"
            })));
        }
    };

    let result = match agent.execute(session_id).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Agent execution failed: {}", e);
            let _ = storage.update_session(Session {
                id: Some(session_id),
                agent_type: "financial-extractor".to_string(),
                objective: "".to_string(),
                context_data: None,
                status: "failed".to_string(),
                result: Some(e.to_string()),
            }).await;
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Agent execution failed",
                "details": e.to_string()
            })));
        }
    };

    let _ = storage.update_session(Session {
        id: Some(session_id),
        agent_type: "financial-extractor".to_string(),
        objective: "".to_string(),
        context_data: None,
        status: "completed".to_string(),
        result: Some(result.clone()),
    }).await;

    Ok(HttpResponse::Ok().json(GeneratePatternResponse {
        session_id,
        status: "completed".to_string(),
        pattern_id: None,
        extracted_data: vec![],
    }))
}
