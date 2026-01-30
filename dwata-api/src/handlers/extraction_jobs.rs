use std::sync::Arc;

use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::extraction_job::{
    CreateExtractionJobRequest, ExtractionJobListResponse,
};

use crate::database::extraction_jobs as db;
use crate::jobs::extraction_manager::ExtractionManager;
use crate::database::AsyncDbConnection;

pub async fn create_extraction_job(
    db_conn: web::Data<AsyncDbConnection>,
    request: web::Json<CreateExtractionJobRequest>,
) -> ActixResult<HttpResponse> {
    let job = db::insert_extraction_job(db_conn.as_ref().clone(), &request)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Created().json(job))
}

pub async fn list_extraction_jobs(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let jobs = db::list_extraction_jobs(db_conn.as_ref().clone(), 50)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ExtractionJobListResponse { jobs }))
}

pub async fn get_extraction_job(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    let job = db::get_extraction_job(db_conn.as_ref().clone(), job_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(job))
}

pub async fn start_extraction(
    manager: web::Data<Arc<ExtractionManager>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    manager
        .start_job(job_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "started" })))
}
