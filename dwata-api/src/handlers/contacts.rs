use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::{ContactsResponse, ContactLinksResponse};
use std::sync::Arc;

use crate::database::contacts as contacts_db;
use crate::database::contact_links as links_db;
use crate::database::Database;

pub async fn list_contacts(
    db: web::Data<Arc<Database>>,
) -> ActixResult<HttpResponse> {
    let contacts = contacts_db::list_contacts(db.async_connection.clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ContactsResponse { contacts }))
}

pub async fn get_contact(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let contact_id = path.into_inner();

    let contact = contacts_db::get_contact(db.async_connection.clone(), contact_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(contact))
}

pub async fn get_contact_links(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let contact_id = path.into_inner();

    let links = links_db::get_contact_links(db.async_connection.clone(), contact_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ContactLinksResponse { links }))
}
