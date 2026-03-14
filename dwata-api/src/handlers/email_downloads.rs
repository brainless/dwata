use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::download::{
    PauseEmailSyncRequest, ResumeEmailSyncRequest, TriggerAllEmailSyncRequest,
    TriggerEmailSyncRequest,
};
use std::sync::Arc;

use crate::jobs::email_sync_manager::EmailSyncManager;

pub async fn trigger_sync(
    manager: web::Data<Arc<EmailSyncManager>>,
    request: web::Json<TriggerEmailSyncRequest>,
) -> ActixResult<HttpResponse> {
    manager
        .sync_credential(request.credential_id, request.direction.clone())
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "status": "started",
        "credential_id": request.credential_id,
        "direction": request.direction,
    })))
}

pub async fn trigger_sync_all(
    manager: web::Data<Arc<EmailSyncManager>>,
    request: web::Json<TriggerAllEmailSyncRequest>,
) -> ActixResult<HttpResponse> {
    use shared_types::download::EmailSyncDirection;
    match request.direction {
        EmailSyncDirection::Recent => manager
            .sync_all_recent()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?,
        EmailSyncDirection::Backfill => manager
            .sync_all_backfill()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?,
    }

    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "status": "started",
        "direction": request.direction,
    })))
}

pub async fn pause_sync(
    manager: web::Data<Arc<EmailSyncManager>>,
    request: web::Json<PauseEmailSyncRequest>,
) -> ActixResult<HttpResponse> {
    manager
        .pause_credential(request.credential_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "paused",
        "credential_id": request.credential_id,
    })))
}

pub async fn resume_sync(
    manager: web::Data<Arc<EmailSyncManager>>,
    request: web::Json<ResumeEmailSyncRequest>,
) -> ActixResult<HttpResponse> {
    manager
        .resume_credential(request.credential_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "resumed",
        "credential_id": request.credential_id,
    })))
}
