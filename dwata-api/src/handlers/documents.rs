use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use shared_types::{
    DocumentKind, DocumentSortBy, ListDocumentsRequest, SearchDocumentsRequest,
    SearchDocumentsResponse, SearchField, SearchTerm,
};
use std::sync::Arc;

use crate::database::documents as documents_db;
use crate::database::Database;
use crate::search::tantivy::TantivySearchIndex;

fn parse_q_term(
    q: &str,
    explicit_field: Option<SearchField>,
    is_phrase: bool,
) -> Option<SearchTerm> {
    let trimmed = q.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(field) = explicit_field {
        return Some(SearchTerm {
            field,
            value: trimmed.to_string(),
            is_phrase,
        });
    }

    if let Some((prefix, value)) = trimmed.split_once(':') {
        let field = match prefix.trim().to_ascii_lowercase().as_str() {
            "from" | "sender" => Some(SearchField::FromAddress),
            "subject" | "title" => Some(SearchField::Title),
            "body" | "text" => Some(SearchField::BodyText),
            "attachment" | "attachments" => Some(SearchField::AttachmentText),
            _ => None,
        };

        if let Some(field) = field {
            let value = value.trim();
            if !value.is_empty() {
                return Some(SearchTerm {
                    field,
                    value: value.to_string(),
                    is_phrase,
                });
            }
        }
    }

    Some(SearchTerm {
        field: SearchField::Any,
        value: trimmed.to_string(),
        is_phrase,
    })
}

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SearchDocumentsQuery {
    pub q: Option<String>,
    pub terms: Option<String>,
    pub field: Option<SearchField>,
    pub is_phrase: Option<bool>,
    pub kind: Option<DocumentKind>,
    pub source_id: Option<i64>,
    pub credential_id: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
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

pub async fn search_documents(
    db: web::Data<Arc<Database>>,
    search_index: web::Data<Arc<TantivySearchIndex>>,
    query: web::Query<SearchDocumentsQuery>,
) -> ActixResult<HttpResponse> {
    let query = query.into_inner();
    let terms = if let Some(terms_json) = query.terms.as_ref().filter(|s| !s.trim().is_empty()) {
        serde_json::from_str::<Vec<SearchTerm>>(terms_json)
            .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid terms: {e}")))?
    } else if let Some(q) = query.q.as_ref().filter(|s| !s.trim().is_empty()) {
        parse_q_term(q, query.field, query.is_phrase.unwrap_or(false))
            .map(|term| vec![term])
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if terms.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(shared_types::ErrorResponse {
                error: "terms must not be empty".to_string(),
            }),
        );
    }

    let request = SearchDocumentsRequest {
        terms,
        kind: query.kind,
        source_id: query.source_id,
        credential_id: query.credential_id,
        limit: query.limit,
        offset: query.offset,
    };

    if request.limit.unwrap_or(25) > 100 {
        return Ok(
            HttpResponse::BadRequest().json(shared_types::ErrorResponse {
                error: "limit must be <= 100".to_string(),
            }),
        );
    }

    let search_result = search_index
        .search(&request)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let ids: Vec<i64> = search_result.hits.iter().map(|h| h.document_id).collect();

    let docs = documents_db::get_documents_by_ids(db.async_connection.clone(), &ids)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let docs_by_id = docs
        .into_iter()
        .map(|d| (d.id, d))
        .collect::<std::collections::HashMap<_, _>>();
    let documents = ids
        .iter()
        .filter_map(|id| docs_by_id.get(id).cloned())
        .collect::<Vec<_>>();
    let missing_ids = ids
        .iter()
        .filter(|id| !docs_by_id.contains_key(id))
        .copied()
        .collect::<Vec<_>>();
    if !missing_ids.is_empty() {
        tracing::warn!(
            missing_count = missing_ids.len(),
            sample_ids = ?missing_ids.iter().take(5).copied().collect::<Vec<_>>(),
            "Search hydration missing documents for some indexed IDs"
        );
    }

    Ok(HttpResponse::Ok().json(SearchDocumentsResponse {
        hits: search_result.hits,
        documents,
        total_hits: search_result.total_hits,
    }))
}
