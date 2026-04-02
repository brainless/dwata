use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::{ContactLinksResponse, PersonsWithCountsResponse};
use std::sync::Arc;

use crate::database::contact_links as links_db;
use crate::database::persons as db;
use crate::database::Database;

pub async fn list_persons(database: web::Data<Arc<Database>>) -> ActixResult<HttpResponse> {
    let persons = db::list_persons_with_counts(database.async_connection.clone(), 500)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(PersonsWithCountsResponse { persons }))
}

pub async fn get_person(
    database: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let person_id = path.into_inner();

    let person = db::get_person(database.async_connection.clone(), person_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(person))
}

pub async fn get_person_links(
    database: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let person_id = path.into_inner();

    let links = links_db::get_person_links(database.async_connection.clone(), person_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ContactLinksResponse { links }))
}
