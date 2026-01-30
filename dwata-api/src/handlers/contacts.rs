use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::ContactsResponse;

use crate::database::contacts as db;
use crate::database::AsyncDbConnection;

pub async fn list_contacts(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let contacts = db::list_contacts(db_conn.as_ref().clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ContactsResponse { contacts }))
}

pub async fn get_contact(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let contact_id = path.into_inner();

    let contact = db::get_contact(db_conn.as_ref().clone(), contact_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(contact))
}
