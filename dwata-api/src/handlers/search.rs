use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use shared_types::{HitId, SearchField, SearchRequest, SearchResponse, SearchTarget, SearchTerm};
use std::sync::Arc;

use crate::database::emails as emails_db;
use crate::database::Database;
use crate::search::tantivy::TantivySearchIndex;

fn render_hit_id(hit_id: &HitId) -> String {
    match hit_id {
        HitId::Email(id) => format!("email:{id}"),
        HitId::File(id) => format!("file:{id}"),
    }
}

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
            "to" | "recipient" => Some(SearchField::ToAddresses),
            "subject" => Some(SearchField::Subject),
            "body" | "text" => Some(SearchField::BodyText),
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
#[serde(rename_all = "snake_case")]
pub struct SearchQuery {
    pub q: Option<String>,
    pub terms: Option<String>,
    pub field: Option<SearchField>,
    pub is_phrase: Option<bool>,
    pub target: Option<SearchTarget>,
    pub credential_id: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub async fn search(
    db: web::Data<Arc<Database>>,
    search_index: web::Data<Arc<TantivySearchIndex>>,
    query: web::Query<SearchQuery>,
) -> ActixResult<HttpResponse> {
    let query = query.into_inner();
    let raw_q = query.q.clone();
    let raw_terms = query.terms.clone();
    let explicit_field = query.field.clone();
    let is_phrase = query.is_phrase.unwrap_or(false);
    let requested_target = query.target.clone();
    let credential_id = query.credential_id;
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);

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
        tracing::warn!(
            q = ?raw_q,
            terms = ?raw_terms,
            "Rejecting search request because parsed terms are empty"
        );
        return Ok(
            HttpResponse::BadRequest().json(shared_types::ErrorResponse {
                error: "terms must not be empty".to_string(),
            }),
        );
    }

    let request = SearchRequest {
        target: query.target.unwrap_or(SearchTarget::Email),
        terms,
        credential_id: query.credential_id,
        limit: query.limit,
        offset: query.offset,
    };

    tracing::info!(
        q = ?raw_q,
        terms_json = ?raw_terms,
        explicit_field = ?explicit_field,
        is_phrase,
        target = ?requested_target,
        effective_target = ?request.target,
        credential_id,
        limit,
        offset,
        parsed_terms = ?request.terms,
        "Search request received"
    );

    if request.limit.unwrap_or(25) > 100 {
        tracing::warn!(
            requested_limit = request.limit.unwrap_or(25),
            "Rejecting search request because limit is > 100"
        );
        return Ok(
            HttpResponse::BadRequest().json(shared_types::ErrorResponse {
                error: "limit must be <= 100".to_string(),
            }),
        );
    }

    let search_result = search_index
        .search(&request)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let sample_hit_ids: Vec<String> = search_result
        .hits
        .iter()
        .take(10)
        .map(|hit| render_hit_id(&hit.hit_id))
        .collect();

    // Extract email IDs from hits
    let email_ids: Vec<i64> = search_result
        .hits
        .iter()
        .filter_map(|h| match &h.hit_id {
            HitId::Email(id) => Some(*id),
            _ => None,
        })
        .collect();

    tracing::info!(
        total_hits = search_result.total_hits,
        page_hits = search_result.hits.len(),
        email_hit_ids = email_ids.len(),
        sample_hit_ids = ?sample_hit_ids,
        "Search index returned hits"
    );

    // Fetch full email data for the hits
    let emails = emails_db::get_emails_by_ids(db.async_connection.clone(), &email_ids)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    // Create a map for quick lookup
    let emails_by_id = emails
        .into_iter()
        .map(|e| (e.id, e))
        .collect::<std::collections::HashMap<_, _>>();

    // Check for missing emails
    let missing_ids: Vec<i64> = email_ids
        .iter()
        .filter(|id| !emails_by_id.contains_key(id))
        .copied()
        .collect();
    if !missing_ids.is_empty() {
        tracing::warn!(
            missing_count = missing_ids.len(),
            sample_ids = ?missing_ids.iter().take(5).copied().collect::<Vec<_>>(),
            "Search hydration missing emails for some indexed IDs"
        );
    }

    tracing::info!(
        requested_email_hits = email_ids.len(),
        hydrated_emails = emails_by_id.len(),
        missing_hydration = missing_ids.len(),
        "Search hydration completed"
    );

    // Update hits with email data where available
    let hits: Vec<shared_types::SearchHit> = search_result
        .hits
        .into_iter()
        .map(|mut hit| {
            if let HitId::Email(email_id) = &hit.hit_id {
                if let Some(email) = emails_by_id.get(email_id) {
                    // Fill in any missing preview fields from the email
                    if hit.subject.is_none() {
                        hit.subject.clone_from(&email.subject);
                    }
                    if hit.from_address.is_none() {
                        hit.from_address = Some(email.from_address.clone());
                    }
                    if hit.date_received.is_none() {
                        hit.date_received = Some(email.date_received);
                    }
                }
            }
            hit
        })
        .collect();

    tracing::info!(
        response_hits = hits.len(),
        response_total_hits = search_result.total_hits,
        "Search response ready"
    );

    Ok(HttpResponse::Ok().json(SearchResponse {
        hits,
        total_hits: search_result.total_hits,
    }))
}
