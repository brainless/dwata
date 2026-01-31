use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::{ListEmailsRequest, ListEmailsResponse};

use crate::database::emails as emails_db;
use crate::database::AsyncDbConnection;

pub async fn list_emails(
    db_conn: web::Data<AsyncDbConnection>,
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

    let emails = emails_db::list_emails(db_conn.as_ref().clone(), folder.as_deref(), limit, offset)
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
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let email_id = path.into_inner();

    let email = emails_db::get_email(db_conn.as_ref().clone(), email_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(email))
}
