use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::{EmailFolder, ListEmailsResponse, ListFoldersResponse};
use std::sync::Arc;

use crate::database::folders as folders_db;
use crate::database::Database;

pub async fn list_folders(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let credential_id = path.into_inner();

    let folders = folders_db::list_folders(db.async_connection.clone(), credential_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ListFoldersResponse { folders }))
}

pub async fn get_folder(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let folder_id = path.into_inner();

    let folder = folders_db::get_folder(db.async_connection.clone(), folder_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(folder))
}

pub async fn list_folder_emails(
    db: web::Data<Arc<Database>>,
    path: web::Path<(i64,)>,
    query: web::Query<PaginationParams>,
) -> ActixResult<HttpResponse> {
    let (folder_id,) = path.into_inner();

    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let emails = crate::database::emails::list_emails(
        db.async_connection.clone(),
        None,
        Some(folder_id),
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
