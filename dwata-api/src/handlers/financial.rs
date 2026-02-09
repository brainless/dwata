use actix_web::{web, HttpResponse, Result as ActixResult};
use crate::database::{
    financial_extraction_attempts as attempts_db,
    financial_extraction_sources as sources_db,
    financial_patterns as patterns_db,
    financial_transactions as db,
};
use crate::database::Database;
use crate::helpers::pattern_validator;
use crate::jobs::financial_extraction_manager::FinancialExtractionManager;
use serde::Deserialize;
use shared_types::{
    FinancialEmailScanRequest, FinancialEmailScanResponse, FinancialEmailScanSender,
    FinancialExtractionAttemptsResponse, FinancialExtractionSummary, FinancialPattern,
};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::info;

pub async fn list_transactions(
    db: web::Data<Arc<Database>>,
) -> ActixResult<HttpResponse> {
    let transactions = db::list_financial_transactions(db.async_connection.clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "transactions": transactions
    })))
}

#[derive(Deserialize)]
pub struct SummaryQuery {
    start_date: String,
    end_date: String,
}

pub async fn get_summary(
    db: web::Data<Arc<Database>>,
    query: web::Query<SummaryQuery>,
) -> ActixResult<HttpResponse> {
    let summary = db::get_financial_summary(
        db.async_connection.clone(),
        &query.start_date,
        &query.end_date,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(summary))
}

#[derive(Deserialize)]
pub struct ExtractionRequest {
    email_ids: Option<Vec<i64>>,
    credential_id: Option<i64>,
}

pub async fn trigger_extraction(
    manager: web::Data<Arc<FinancialExtractionManager>>,
    request: web::Json<ExtractionRequest>,
) -> ActixResult<HttpResponse> {
    info!(
        "Triggering financial extraction: email_ids={:?}, credential_id={:?}",
        request.email_ids, request.credential_id
    );

    let count = manager
        .extract_from_emails(request.email_ids.clone(), request.credential_id.clone())
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    info!(
        "Financial extraction completed: extracted_count={}",
        count
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "extracted_count": count,
        "status": "completed"
    })))
}

pub async fn get_extraction_summary(
    db: web::Data<Arc<Database>>,
) -> ActixResult<HttpResponse> {
    let summary: FinancialExtractionSummary = sources_db::get_extraction_summary(
        db.async_connection.clone(),
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(summary))
}

pub async fn list_extraction_attempts(
    db: web::Data<Arc<Database>>,
) -> ActixResult<HttpResponse> {
    let attempts = attempts_db::list_attempts(db.async_connection.clone(), 50)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(FinancialExtractionAttemptsResponse {
        attempts,
    }))
}

#[derive(Deserialize)]
pub struct ListPatternsQuery {
    active_only: Option<bool>,
    is_default: Option<bool>,
    document_type: Option<String>,
}

pub async fn list_patterns(
    db: web::Data<Arc<Database>>,
    query: web::Query<ListPatternsQuery>,
) -> ActixResult<HttpResponse> {
    let patterns = patterns_db::list_patterns(
        db.async_connection.clone(),
        query.active_only.unwrap_or(true),
        query.is_default,
        query.document_type.clone(),
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "patterns": patterns,
        "total": patterns.len()
    })))
}

pub async fn get_pattern(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let pattern = patterns_db::get_pattern(db.async_connection.clone(), *path)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "pattern": pattern
    })))
}

#[derive(Deserialize)]
pub struct CreatePatternRequest {
    pub name: String,
    pub regex_pattern: String,
    pub description: Option<String>,
    pub sender_email: Option<String>,
    pub document_type: String,
    pub status: String,
    pub confidence: f32,
    pub amount_group: usize,
    pub vendor_group: Option<usize>,
    pub source_vendor_group: Option<usize>,
    pub destination_vendor_group: Option<usize>,
    pub date_group: Option<usize>,
    pub reference_group: Option<usize>,
    pub is_active: Option<bool>,
}

pub async fn create_pattern(
    db: web::Data<Arc<Database>>,
    request: web::Json<CreatePatternRequest>,
) -> ActixResult<HttpResponse> {
    pattern_validator::validate_pattern(
        &request.name,
        &request.regex_pattern,
        request.amount_group,
        request.vendor_group,
        request.source_vendor_group,
        request.destination_vendor_group,
        request.date_group,
        request.reference_group,
        request.confidence,
        &request.document_type,
        &request.status,
    )
    .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    let name_exists = patterns_db::pattern_name_exists(
        db.async_connection.clone(),
        &request.name,
        None,
    )
    .await
    .unwrap_or(false);

    if name_exists {
        return Err(actix_web::error::ErrorBadRequest("Pattern name already exists"));
    }

    let regex_exists = patterns_db::pattern_regex_exists(
        db.async_connection.clone(),
        &request.regex_pattern,
        request.sender_email.as_deref(),
        None,
    )
    .await
    .unwrap_or(false);

    if regex_exists {
        return Err(actix_web::error::ErrorBadRequest(
            "Pattern with this regex already exists"
        ));
    }

    let pattern = FinancialPattern {
        id: 0,
        name: request.name.clone(),
        regex_pattern: request.regex_pattern.clone(),
        description: request.description.clone(),
        sender_email: request.sender_email.clone(),
        document_type: request.document_type.clone(),
        status: request.status.clone(),
        confidence: request.confidence,
        amount_group: request.amount_group,
        vendor_group: request.vendor_group,
        source_vendor_group: request.source_vendor_group,
        destination_vendor_group: request.destination_vendor_group,
        date_group: request.date_group,
        reference_group: request.reference_group,
        is_default: false,
        is_active: request.is_active.unwrap_or(true),
        match_count: 0,
        last_matched_at: None,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    let id = patterns_db::insert_pattern(db.async_connection.clone(), &pattern)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let created_pattern = patterns_db::get_pattern(db.async_connection.clone(), id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "pattern": created_pattern,
        "message": "Pattern created successfully"
    })))
}

#[derive(Deserialize)]
pub struct UpdatePatternRequest {
    pub name: Option<String>,
    pub regex_pattern: Option<String>,
    pub description: Option<String>,
    pub sender_email: Option<String>,
    pub document_type: Option<String>,
    pub status: Option<String>,
    pub confidence: Option<f32>,
    pub amount_group: Option<usize>,
    pub vendor_group: Option<usize>,
    pub source_vendor_group: Option<usize>,
    pub destination_vendor_group: Option<usize>,
    pub date_group: Option<usize>,
    pub reference_group: Option<usize>,
    pub is_active: Option<bool>,
}

pub async fn update_pattern(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
    request: web::Json<UpdatePatternRequest>,
) -> ActixResult<HttpResponse> {
    let pattern_id = *path;

    let existing = patterns_db::get_pattern(db.async_connection.clone(), pattern_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    if existing.is_default {
        if request.name.is_some() || request.regex_pattern.is_some() {
            return Err(actix_web::error::ErrorForbidden(
                "Cannot modify name or regex of default patterns"
            ));
        }
    }

    let updated = FinancialPattern {
        id: pattern_id,
        name: request.name.clone().unwrap_or(existing.name),
        regex_pattern: request.regex_pattern.clone().unwrap_or(existing.regex_pattern),
        description: request.description.clone().or(existing.description),
        sender_email: request.sender_email.clone().or(existing.sender_email),
        document_type: request.document_type.clone().unwrap_or(existing.document_type),
        status: request.status.clone().unwrap_or(existing.status),
        confidence: request.confidence.unwrap_or(existing.confidence),
        amount_group: request.amount_group.unwrap_or(existing.amount_group),
        vendor_group: request.vendor_group.or(existing.vendor_group),
        source_vendor_group: request
            .source_vendor_group
            .or(existing.source_vendor_group),
        destination_vendor_group: request
            .destination_vendor_group
            .or(existing.destination_vendor_group),
        date_group: request.date_group.or(existing.date_group),
        reference_group: request.reference_group.or(existing.reference_group),
        is_default: existing.is_default,
        is_active: request.is_active.unwrap_or(existing.is_active),
        match_count: existing.match_count,
        last_matched_at: existing.last_matched_at,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now().timestamp(),
    };

    pattern_validator::validate_pattern(
        &updated.name,
        &updated.regex_pattern,
        updated.amount_group,
        updated.vendor_group,
        updated.source_vendor_group,
        updated.destination_vendor_group,
        updated.date_group,
        updated.reference_group,
        updated.confidence,
        &updated.document_type,
        &updated.status,
    )
    .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    patterns_db::update_pattern(db.async_connection.clone(), pattern_id, &updated)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let updated_pattern = patterns_db::get_pattern(db.async_connection.clone(), pattern_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "pattern": updated_pattern,
        "message": "Pattern updated successfully"
    })))
}

const DEFAULT_FINANCIAL_KEYWORDS: &[&str] = &[
    "payment",
    "paid",
    "invoice",
    "receipt",
    "transaction",
    "transfer",
    "deposit",
    "withdrawal",
    "charge",
    "charged",
    "refund",
    "statement",
    "balance",
    "credit",
    "debit",
];

const TEMPLATE_SIMILARITY_THRESHOLD: f32 = 0.45;

fn build_fts_query(keywords: &[&str]) -> String {
    let mut parts = Vec::with_capacity(keywords.len());
    for keyword in keywords {
        let escaped = keyword.replace('"', " ");
        if !escaped.trim().is_empty() {
            parts.push(escaped);
        }
    }
    parts.join(" OR ")
}

pub async fn scan_financial_emails(
    db: web::Data<Arc<Database>>,
    request: web::Json<FinancialEmailScanRequest>,
) -> ActixResult<HttpResponse> {
    let fts_query = build_fts_query(DEFAULT_FINANCIAL_KEYWORDS);

    let total_emails = crate::database::emails::count_emails(
        db.async_connection.clone(),
        request.credential_id,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let rows = crate::database::emails::list_email_scan_rows_fts(
        db.async_connection.clone(),
        request.credential_id,
        request.max_emails,
        &fts_query,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let mut sender_counts: HashMap<String, i64> = HashMap::new();
    let mut sender_tokens: HashMap<String, Vec<Vec<String>>> = HashMap::new();

    for row in rows.iter() {
        let mut content = String::new();
        if let Some(subject) = &row.subject {
            content.push_str(subject);
            content.push('\n');
        }
        if let Some(body_text) = &row.body_text {
            content.push_str(body_text);
            content.push('\n');
        }
        if let Some(body_html) = &row.body_html {
            content.push_str(body_html);
        }

        let sender = row.from_address.clone();
        *sender_counts.entry(sender.clone()).or_insert(0) += 1;
        sender_tokens
            .entry(sender)
            .or_insert_with(Vec::new)
            .push(tokenize_words(&content));
    }

    // If a sender's matched emails differ too much at a word level, they are likely not
    // templated transaction emails. We drop them for now (may miss manual emails).
    let mut total_matched = 0i64;
    sender_counts.retain(|sender, count| {
        let keep = sender_tokens
            .get(sender)
            .and_then(|tokens| {
                if tokens.len() < 2 {
                    return Some(true);
                }
                let avg = average_normalized_distance(tokens);
                Some(avg <= TEMPLATE_SIMILARITY_THRESHOLD)
            })
            .unwrap_or(true);

        if keep {
            total_matched += *count;
        }
        keep
    });

    let mut senders: Vec<FinancialEmailScanSender> = sender_counts
        .into_iter()
        .map(|(sender_email, matched_count)| FinancialEmailScanSender {
            sender_email,
            matched_count,
        })
        .collect();

    senders.sort_by(|a, b| {
        b.matched_count
            .cmp(&a.matched_count)
            .then_with(|| a.sender_email.cmp(&b.sender_email))
    });

    if let Some(max_senders) = request.max_senders {
        if senders.len() > max_senders {
            senders.truncate(max_senders);
        }
    }

    Ok(HttpResponse::Ok().json(FinancialEmailScanResponse {
        total_emails,
        total_emails_scanned: rows.len() as i64,
        total_matched_emails: total_matched,
        senders,
    }))
}

fn tokenize_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            words.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn average_normalized_distance(samples: &[Vec<String>]) -> f32 {
    let baseline = &samples[0];
    let mut total = 0f32;
    let mut count = 0f32;
    for candidate in samples.iter().skip(1) {
        total += normalized_word_distance(baseline, candidate);
        count += 1.0;
    }
    if count == 0.0 {
        0.0
    } else {
        total / count
    }
}

fn normalized_word_distance(a: &[String], b: &[String]) -> f32 {
    let dist = word_edit_distance(a, b) as f32;
    let denom = a.len().max(b.len()).max(1) as f32;
    dist / denom
}

fn word_edit_distance(a: &[String], b: &[String]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];

    for (i, aw) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, bw) in b.iter().enumerate() {
            let cost = if aw == bw { 0 } else { 1 };
            let insert = curr[j] + 1;
            let delete = prev[j + 1] + 1;
            let replace = prev[j] + cost;
            curr[j + 1] = insert.min(delete).min(replace);
        }
        prev.copy_from_slice(&curr);
    }

    prev[b.len()]
}

#[derive(Deserialize)]
pub struct TogglePatternRequest {
    pub is_active: bool,
}

pub async fn toggle_pattern(
    db: web::Data<Arc<Database>>,
    path: web::Path<i64>,
    request: web::Json<TogglePatternRequest>,
) -> ActixResult<HttpResponse> {
    let pattern_id = *path;

    patterns_db::toggle_pattern_active(
        db.async_connection.clone(),
        pattern_id,
        request.is_active,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let pattern = patterns_db::get_pattern(db.async_connection.clone(), pattern_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let message = if request.is_active {
        "Pattern enabled successfully"
    } else {
        "Pattern disabled successfully"
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "pattern": pattern,
        "message": message
    })))
}
