use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::EventsResponse;
use std::sync::Arc;

use crate::database::events as db;
use crate::database::Database;

pub async fn list_events(
    database: web::Data<Arc<Database>>,
) -> ActixResult<HttpResponse> {
    let events = db::list_events(database.async_connection.clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(EventsResponse { events }))
}

pub async fn get_event(
    database: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let event_id = path.into_inner();

    let event = db::get_event(database.async_connection.clone(), event_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(event))
}
