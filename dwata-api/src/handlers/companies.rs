use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::CompaniesResponse;
use std::sync::Arc;

use crate::database::companies as db;
use crate::database::Database;

pub async fn list_companies(
    database: web::Data<Arc<Database>>,
) -> ActixResult<HttpResponse> {
    let companies = db::list_companies(database.async_connection.clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(CompaniesResponse { companies }))
}

pub async fn get_company(
    database: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let company_id = path.into_inner();

    let company = db::get_company(database.async_connection.clone(), company_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(company))
}
