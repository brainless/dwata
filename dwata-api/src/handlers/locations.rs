use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::LocationsResponse;
use std::sync::Arc;

use crate::database::locations as db;
use crate::database::Database;

pub async fn list_locations(database: web::Data<Arc<Database>>) -> ActixResult<HttpResponse> {
    let locations = db::list_locations(database.async_connection.clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(LocationsResponse { locations }))
}

pub async fn get_location(
    database: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let location_id = path.into_inner();

    let location = db::get_location(database.async_connection.clone(), location_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(location))
}
