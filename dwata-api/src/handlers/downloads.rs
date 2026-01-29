use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use std::sync::Arc;
use shared_types::download::{
    CreateDownloadJobRequest, DownloadJobListResponse, DownloadJobStatus,
};

use crate::database::downloads as db;
use crate::database::Database;
use crate::jobs::download_manager::DownloadManager;

pub async fn create_download_job(
    db: web::Data<Arc<Database>>,
    request: web::Json<CreateDownloadJobRequest>,
) -> ActixResult<HttpResponse> {
    let job = db::insert_download_job(db.async_connection.clone(), &request)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Created().json(job))
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

    let _ = manager.pause_job(job_id).await;

    db::update_job_status(
        db.async_connection.clone(),
        job_id,
        DownloadJobStatus::Cancelled,
        None,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::NoContent().finish())
}
