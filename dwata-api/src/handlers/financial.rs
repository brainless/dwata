use crate::config::ApiConfig;
use crate::database::{
    financial_bills as bills_db, financial_templates as templates_db,
    financial_transactions as transactions_db, Database,
};
use crate::search::tantivy::TantivySearchIndex;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use shared_types::{
    BillStatus, DeleteFinancialTemplatesRequest, DeleteFinancialTemplatesResponse,
    FinancialExtractionTemplate, FinancialPagination, FinancialSummary,
    FinancialTemplateFieldMapping, FinancialTemplateWithVariables, ListFinancialBillsResponse,
    ListFinancialTemplatesResponse,
};
use std::sync::Arc;
#[derive(Deserialize)]
pub struct ExtractFinancialRequest {
    #[serde(default)]
    pub credential_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct TransactionFilters {
    #[serde(default)]
    pub payer_vendor_id: Option<i64>,
    #[serde(default)]
    pub payee_vendor_id: Option<i64>,
    #[serde(default)]
    pub bill_id: Option<i64>,
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

#[derive(Deserialize)]
pub struct BillFilters {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub start_due_date: Option<String>,
    #[serde(default)]
    pub end_due_date: Option<String>,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct TemplatesQuery {
    #[serde(default = "default_templates_limit")]
    pub limit: usize,
}

fn default_templates_limit() -> usize {
    200
}

pub async fn list_transactions(
    db: web::Data<Arc<Database>>,
    query: web::Query<TransactionFilters>,
) -> ActixResult<HttpResponse> {
    let offset = (query.page.saturating_sub(1)) * query.limit;
    let start_date_ms = query
        .start_date
        .as_deref()
        .map(bills_db::date_string_to_utc_ms)
        .transpose()
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
    let end_date_ms = query
        .end_date
        .as_deref()
        .map(bills_db::date_string_to_utc_ms)
        .transpose()
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    let (transactions, total_count) = transactions_db::list_financial_transactions_filtered(
        &db.sqlx_pool,
        query.payer_vendor_id,
        query.payee_vendor_id,
        query.bill_id,
        start_date_ms,
        end_date_ms,
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

pub async fn list_bills(
    db: web::Data<Arc<Database>>,
    query: web::Query<BillFilters>,
) -> ActixResult<HttpResponse> {
    let offset = (query.page.saturating_sub(1)) * query.limit;
    let parsed_status = match query.status.as_deref() {
        Some("received") => Some(BillStatus::Received),
        Some("unpaid") => Some(BillStatus::Unpaid),
        Some("paid") => Some(BillStatus::Paid),
        Some("overdue") => Some(BillStatus::Overdue),
        Some("cancelled") => Some(BillStatus::Cancelled),
        Some(other) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Unsupported bill status filter: {other}")
            })))
        }
        None => None,
    };

    let start_due_date_ms = query
        .start_due_date
        .as_deref()
        .map(bills_db::date_string_to_utc_ms)
        .transpose()
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
    let end_due_date_ms = query
        .end_due_date
        .as_deref()
        .map(bills_db::date_string_to_utc_ms)
        .transpose()
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    let (bills, total_count) = bills_db::list_financial_bills_filtered(
        &db.sqlx_pool,
        parsed_status,
        start_due_date_ms,
        end_due_date_ms,
        query.limit,
        offset,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let total_pages = (total_count as f64 / query.limit as f64).ceil() as usize;

    Ok(HttpResponse::Ok().json(ListFinancialBillsResponse {
        bills,
        pagination: FinancialPagination {
            page: query.page,
            limit: query.limit,
            total_count,
            total_pages,
        },
    }))
}

pub async fn list_templates(
    db: web::Data<Arc<Database>>,
    query: web::Query<TemplatesQuery>,
) -> ActixResult<HttpResponse> {
    let limit = query.limit.clamp(1, 500);
    let rows =
        templates_db::list_active_templates_with_variables(db.async_connection.clone(), limit)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let templates = rows
        .into_iter()
        .map(|row| FinancialTemplateWithVariables {
            template: FinancialExtractionTemplate {
                id: row.id,
                data_source_type: row.data_source_type,
                data_source_id: row.data_source_id,
                template_type: row.template_type,
                template_body: row.template_body,
                status: row.status,
                version: row.version,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            variables: row
                .variables
                .into_iter()
                .map(|v| FinancialTemplateFieldMapping {
                    placeholder_name: v.placeholder_name,
                    target_field: v.target_field,
                })
                .collect(),
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(ListFinancialTemplatesResponse { templates }))
}

pub async fn delete_templates(
    db: web::Data<Arc<Database>>,
    request: web::Json<DeleteFinancialTemplatesRequest>,
) -> ActixResult<HttpResponse> {
    let deleted_count =
        templates_db::delete_templates_by_ids(db.async_connection.clone(), &request.template_ids)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(DeleteFinancialTemplatesResponse { deleted_count }))
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
        transactions_db::get_financial_summary(&db.sqlx_pool, &query.start_date, &query.end_date)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(summary))
}

pub async fn extract_financial(
    db: web::Data<Arc<Database>>,
    config: web::Data<Arc<crate::config::ApiConfig>>,
    request: web::Json<ExtractFinancialRequest>,
) -> ActixResult<HttpResponse> {
    let result = crate::helpers::financial_extraction::extract_financial_from_templates(
        db.get_ref().clone(),
        config.get_ref(),
        request.credential_id,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_bill(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let bill_id = path.into_inner();

    let bill = bills_db::get_financial_bill(&db.sqlx_pool, bill_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    match bill {
        Some(bill) => Ok(HttpResponse::Ok().json(bill)),
        None => Err(actix_web::error::ErrorNotFound("Bill not found".to_string()).into()),
    }
}

pub async fn get_transaction(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let transaction_id = path.into_inner();

    let transaction = transactions_db::get_financial_transaction(&db.sqlx_pool, transaction_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    match transaction {
        Some(transaction) => Ok(HttpResponse::Ok().json(transaction)),
        None => Err(actix_web::error::ErrorNotFound("Transaction not found".to_string()).into()),
    }
}
