use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::PositionsResponse;

use crate::database::positions as db;
use crate::database::AsyncDbConnection;

pub async fn list_positions(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let positions = db::list_positions(db_conn.as_ref().clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(PositionsResponse { positions }))
}

pub async fn get_position(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let position_id = path.into_inner();

    let position = db::get_position(db_conn.as_ref().clone(), position_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(position))
}

pub async fn list_contact_positions(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let contact_id = path.into_inner();

    let positions = db::list_contact_positions(db_conn.as_ref().clone(), contact_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(PositionsResponse { positions }))
}
