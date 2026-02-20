use crate::config::ApiConfig;
use crate::database::{financial_transactions as db, Database};
use crate::search::tantivy::TantivySearchIndex;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use shared_types::{DetectFinancialTemplatesRequest, FinancialSummary};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct TransactionFilters {
    #[serde(default)]
    pub source_vendor_id: Option<i64>,
    #[serde(default)]
    pub destination_vendor_id: Option<i64>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub min_amount: Option<f64>,
    #[serde(default)]
    pub max_amount: Option<f64>,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_page() -> usize {
    1
}

fn default_limit() -> usize {
    500
}

pub async fn list_transactions(
    db: web::Data<Arc<Database>>,
    query: web::Query<TransactionFilters>,
) -> ActixResult<HttpResponse> {
    let offset = (query.page.saturating_sub(1)) * query.limit;

    let (transactions, total_count) = db::list_financial_transactions_filtered(
        &db.sqlx_pool,
        query.source_vendor_id,
        query.destination_vendor_id,
        query.start_date.as_deref(),
        query.end_date.as_deref(),
        query.min_amount,
        query.max_amount,
        query.limit,
        offset,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let total_pages = (total_count as f64 / query.limit as f64).ceil() as usize;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "transactions": transactions,
        "pagination": {
            "page": query.page,
            "limit": query.limit,
            "total_count": total_count,
            "total_pages": total_pages,
        }
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
    let summary: FinancialSummary =
        db::get_financial_summary(&db.sqlx_pool, &query.start_date, &query.end_date)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(summary))
}

pub async fn detect_templates(
    db: web::Data<Arc<Database>>,
    search_index: web::Data<Arc<TantivySearchIndex>>,
    config: web::Data<Arc<ApiConfig>>,
    request: web::Json<DetectFinancialTemplatesRequest>,
) -> ActixResult<HttpResponse> {
    let response = crate::helpers::template_detection::detect_and_store_templates(
        db,
        search_index,
        config,
        request.into_inner(),
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(response))
}
