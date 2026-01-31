use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::{ListEmailsRequest, ListEmailsResponse};
use std::sync::Arc;

use crate::database::emails as emails_db;
use crate::database::Database;

pub async fn list_emails(
    db: web::Data<Arc<Database>>,
    query: web::Query<ListEmailsRequest>,
) -> ActixResult<HttpResponse> {
    let ListEmailsRequest {
        folder,
        limit,
        offset,
        search_query: _,
    } = query.into_inner();

    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    let emails = emails_db::list_emails(db.async_connection.clone(), folder.as_deref(), limit, offset)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let total_count = emails.len() as i64;
    let has_more = emails.len() == limit;

    Ok(HttpResponse::Ok().json(ListEmailsResponse {
        emails,
        total_count,
        has_more,
    }))
}

pub async fn get_email(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let email_id = path.into_inner();

    let email = emails_db::get_email(db.async_connection.clone(), email_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(email))
}
