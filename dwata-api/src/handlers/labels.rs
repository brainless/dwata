use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::{EmailLabel, ListEmailsResponse, ListLabelsResponse};
use std::sync::Arc;

use crate::database::{emails as emails_db, labels as labels_db};
use crate::database::Database;

pub async fn list_labels(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let credential_id = path.into_inner();

    let labels = labels_db::list_labels(db.async_connection.clone(), credential_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ListLabelsResponse { labels }))
}

pub async fn get_label(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let label_id = path.into_inner();

    let label = labels_db::get_label(db.async_connection.clone(), label_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(label))
}

pub async fn list_label_emails(
    db: web::Data<Arc<Database>>,
    path: web::Path<(i64,)>,
    query: web::Query<PaginationParams>,
) -> ActixResult<HttpResponse> {
    let (label_id,) = path.into_inner();

    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let emails = emails_db::list_emails_by_label(
        db.async_connection.clone(),
        label_id,
        limit,
        offset,
    )
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

#[derive(serde::Deserialize)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
