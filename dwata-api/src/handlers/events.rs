use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::EventsResponse;

use crate::database::events as db;
use crate::database::AsyncDbConnection;

pub async fn list_events(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let events = db::list_events(db_conn.as_ref().clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(EventsResponse { events }))
}

pub async fn get_event(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let event_id = path.into_inner();

    let event = db::get_event(db_conn.as_ref().clone(), event_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(event))
}
