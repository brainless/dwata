use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use shared_types::{DocumentKind, DocumentSortBy, ListDocumentsRequest};
use std::sync::Arc;

use crate::database::documents as documents_db;
use crate::database::Database;

#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    pub source_id: Option<i64>,
    pub credential_id: Option<i64>,
    pub kind: Option<DocumentKind>,
    pub parent_document_id: Option<i64>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub cursor_sort_value: Option<i64>,
    pub cursor_id: Option<i64>,
    pub sort_by: Option<DocumentSortBy>,
}

fn parse_cursor(
    query: &ListDocumentsQuery,
) -> Result<Option<shared_types::DocumentCursor>, String> {
    if let Some(cursor) = query.cursor.as_ref().filter(|v| !v.trim().is_empty()) {
        if cursor.contains(':') {
            let mut parts = cursor.splitn(2, ':');
            let sort_value = parts
                .next()
                .ok_or_else(|| "Malformed cursor".to_string())?
                .parse::<i64>()
                .map_err(|_| "Malformed cursor".to_string())?;
            let id = parts
                .next()
                .ok_or_else(|| "Malformed cursor".to_string())?
                .parse::<i64>()
                .map_err(|_| "Malformed cursor".to_string())?;
            return Ok(Some(shared_types::DocumentCursor { sort_value, id }));
        }

        let cursor_json: shared_types::DocumentCursor =
            serde_json::from_str(cursor).map_err(|_| "Malformed cursor".to_string())?;
        return Ok(Some(cursor_json));
    }

    match (query.cursor_sort_value, query.cursor_id) {
        (Some(sort_value), Some(id)) => Ok(Some(shared_types::DocumentCursor { sort_value, id })),
        (None, None) => Ok(None),
        _ => Err("Malformed cursor".to_string()),
    }
}

pub async fn list_documents(
    db: web::Data<Arc<Database>>,
    query: web::Query<ListDocumentsQuery>,
) -> ActixResult<HttpResponse> {
    let query = query.into_inner();
    let limit = query.limit.unwrap_or(50);

    if limit > 200 {
        return Ok(
            HttpResponse::BadRequest().json(shared_types::ErrorResponse {
                error: "limit must be <= 200".to_string(),
            }),
        );
    }

    let cursor = parse_cursor(&query).map_err(actix_web::error::ErrorBadRequest)?;

    let response = documents_db::list_documents(
        db.async_connection.clone(),
        ListDocumentsRequest {
            source_id: query.source_id,
            credential_id: query.credential_id,
            kind: query.kind,
            parent_document_id: query.parent_document_id,
            limit: Some(limit),
            cursor,
            sort_by: query.sort_by,
        },
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(response))
}

pub async fn get_document(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();
    let document = documents_db::get_document(db.async_connection.clone(), id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    match document {
        Some(doc) => Ok(HttpResponse::Ok().json(doc)),
        None => Ok(HttpResponse::NotFound().json(shared_types::ErrorResponse {
            error: format!("Document {} not found", id),
        })),
    }
}
