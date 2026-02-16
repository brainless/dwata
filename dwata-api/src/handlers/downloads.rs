use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use shared_types::download::{
    CreateDownloadJobRequest, DownloadJobListResponse, DownloadJobStatus, JobType,
};
use std::sync::Arc;

use crate::database::downloads as db;
use crate::database::Database;
use crate::jobs::download_manager::DownloadManager;

pub async fn create_download_job(
    db: web::Data<Arc<Database>>,
    manager: web::Data<Arc<DownloadManager>>,
    request: web::Json<CreateDownloadJobRequest>,
    query: web::Query<CreateJobQuery>,
) -> ActixResult<HttpResponse> {
    let job_type = match query.job_type.as_str() {
        "historical-backfill" => JobType::HistoricalBackfill,
        _ => JobType::RecentSync,
    };

    let job = db::insert_download_job(db.async_connection.clone(), &request, job_type)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    // Auto-start the job immediately after creation
    if let Err(e) = manager.start_job(job.id).await {
        tracing::warn!("Failed to auto-start job {}: {}", job.id, e);
        // Don't fail the request - the job was created successfully
        // User can manually start it later if needed
    }

    Ok(HttpResponse::Created().json(job))
}

#[derive(Deserialize)]
pub struct CreateJobQuery {
    #[serde(default)]
    job_type: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

pub async fn list_download_jobs(
    db: web::Data<Arc<Database>>,
    query: web::Query<ListQuery>,
) -> ActixResult<HttpResponse> {
    let jobs = db::list_download_jobs(
        db.async_connection.clone(),
        query.status.as_deref(),
        query.limit,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(DownloadJobListResponse { jobs }))
}

pub async fn get_download_job(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    let job = db::get_download_job(db.async_connection.clone(), job_id)
        .await
        .map_err(|e| match e {
            db::DownloadDbError::NotFound => actix_web::error::ErrorNotFound("Job not found"),
            _ => actix_web::error::ErrorInternalServerError(e.to_string()),
        })?;

    Ok(HttpResponse::Ok().json(job))
}

pub async fn start_download(
    manager: web::Data<Arc<DownloadManager>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    manager
        .start_job(job_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "started" })))
}

pub async fn pause_download(
    manager: web::Data<Arc<DownloadManager>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    manager
        .pause_job(job_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "paused" })))
}

pub async fn delete_download_job(
    db: web::Data<Arc<Database>>,
    manager: web::Data<Arc<DownloadManager>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    // First pause the job if it's running
    let _ = manager.pause_job(job_id).await;

    // Then actually delete it (this will cascade delete download_items)
    db::delete_download_job(db.async_connection.clone(), job_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::NoContent().finish())
}

pub async fn trigger_sync(manager: web::Data<Arc<DownloadManager>>) -> ActixResult<HttpResponse> {
    // Ensure jobs exist for all credentials
    manager
        .ensure_jobs_for_all_credentials()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    // TESTING: Comment out recent sync to test historical backfill only
    // Start all recent sync jobs
    // manager
    //     .sync_all_jobs()
    //     .await
    //     .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    // Start all historical backfill jobs
    manager
        .sync_all_historical_backfill()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "triggered",
        "message": "Historical backfill started for all accounts"
    })))
}
