use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::CompaniesResponse;

use crate::database::companies as db;
use crate::database::AsyncDbConnection;

pub async fn list_companies(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let companies = db::list_companies(db_conn.as_ref().clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(CompaniesResponse { companies }))
}

pub async fn get_company(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let company_id = path.into_inner();

    let company = db::get_company(db_conn.as_ref().clone(), company_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(company))
}
