use actix_web::{web, HttpResponse, Result as ActixResult};
use crate::database::financial_transactions as db;
use crate::database::Database;
use crate::jobs::financial_extraction_manager::FinancialExtractionManager;
use serde::Deserialize;
use std::sync::Arc;

pub async fn list_transactions(
    db: web::Data<Arc<Database>>,
) -> ActixResult<HttpResponse> {
    let transactions = db::list_financial_transactions(db.async_connection.clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "transactions": transactions
    })))
}

#[derive(Deserialize)]
pub struct SummaryQuery {
    start_date: String,
    end_date: String,
}

pub async fn get_summary(
    db: web::Data<Arc<Database>>,
    query: web::Query<SummaryQuery>,
) -> ActixResult<HttpResponse> {
    let summary = db::get_financial_summary(
        db.async_connection.clone(),
        &query.start_date,
        &query.end_date,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(summary))
}

#[derive(Deserialize)]
pub struct ExtractionRequest {
    email_ids: Option<Vec<i64>>,
}

pub async fn trigger_extraction(
    manager: web::Data<Arc<FinancialExtractionManager>>,
    request: web::Json<ExtractionRequest>,
) -> ActixResult<HttpResponse> {
    let count = manager
        .extract_from_emails(request.email_ids.clone())
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "extracted_count": count,
        "status": "completed"
    })))
}
