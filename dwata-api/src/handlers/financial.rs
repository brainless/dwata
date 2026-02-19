use crate::database::{financial_transactions as db, Database};
use crate::financial_keywords::{build_tantivy_query, DEFAULT_FINANCIAL_KEYWORDS};
use crate::search::tantivy::TantivySearchIndex;
use actix_web::{web, HttpResponse, Result as ActixResult};
use rayon::prelude::*;
use serde::Deserialize;
use shared_types::{
    DocumentKind, FinancialEmailScanRequest, FinancialEmailScanResponse, FinancialEmailScanSender,
    SearchDocumentsRequest, SearchField, SearchTerm,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

#[derive(Deserialize)]
pub struct TransactionFilters {
    #[serde(default)]
    pub source_vendor_id: Option<i64>,
    #[serde(default)]
    pub destination_vendor_id: Option<i64>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub min_amount: Option<f64>,
    #[serde(default)]
    pub max_amount: Option<f64>,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_page() -> usize {
    1
}

fn default_limit() -> usize {
    500
}

pub async fn list_transactions(
    db: web::Data<Arc<Database>>,
    query: web::Query<TransactionFilters>,
) -> ActixResult<HttpResponse> {
    let offset = (query.page.saturating_sub(1)) * query.limit;

    let (transactions, total_count) = db::list_financial_transactions_filtered(
        &db.sqlx_pool,
        query.source_vendor_id,
        query.destination_vendor_id,
        query.start_date.as_deref(),
        query.end_date.as_deref(),
        query.min_amount,
        query.max_amount,
        query.limit,
        offset,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let total_pages = (total_count as f64 / query.limit as f64).ceil() as usize;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "transactions": transactions,
        "pagination": {
            "page": query.page,
            "limit": query.limit,
            "total_count": total_count,
            "total_pages": total_pages,
        }
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
    let summary = db::get_financial_summary(&db.sqlx_pool, &query.start_date, &query.end_date)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(summary))
}

const TEMPLATE_SIMILARITY_THRESHOLD: f32 = 0.45;

pub async fn scan_financial_emails(
    db: web::Data<Arc<Database>>,
    search_index: web::Data<Arc<TantivySearchIndex>>,
    request: web::Json<FinancialEmailScanRequest>,
) -> ActixResult<HttpResponse> {
    let scan_started = Instant::now();
    info!(
        credential_id = ?request.credential_id,
        max_emails = ?request.max_emails,
        max_senders = ?request.max_senders,
        "scan_financial_emails started"
    );

    let tantivy_query = build_tantivy_query(DEFAULT_FINANCIAL_KEYWORDS);

    let step_started = Instant::now();
    let total_emails =
        crate::database::emails::count_emails(db.async_connection.clone(), request.credential_id)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    info!(
        duration_ms = step_started.elapsed().as_millis(),
        total_emails, "scan_financial_emails count_emails completed"
    );

    let max_results = request.max_emails.unwrap_or(usize::MAX);
    let mut matched_document_ids = Vec::new();
    let mut seen_document_ids = HashSet::new();
    let mut offset = 0usize;

    let step_started = Instant::now();
    let mut search_pages = 0usize;
    while matched_document_ids.len() < max_results {
        let remaining = max_results.saturating_sub(matched_document_ids.len());
        let page_limit = remaining.min(100);
        if page_limit == 0 {
            break;
        }

        let search_result = search_index
            .search(&SearchDocumentsRequest {
                terms: vec![SearchTerm {
                    field: SearchField::Any,
                    value: tantivy_query.clone(),
                    is_phrase: false,
                }],
                kind: Some(DocumentKind::Email),
                source_id: None,
                credential_id: request.credential_id,
                limit: Some(page_limit),
                offset: Some(offset),
            })
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

        if search_result.hits.is_empty() {
            break;
        }

        for hit in &search_result.hits {
            if seen_document_ids.insert(hit.document_id) {
                matched_document_ids.push(hit.document_id);
            }
        }

        let fetched_count = search_result.hits.len();
        search_pages += 1;
        if fetched_count < page_limit {
            break;
        }
        offset += fetched_count;
    }
    info!(
        duration_ms = step_started.elapsed().as_millis(),
        pages = search_pages,
        matched_document_ids = matched_document_ids.len(),
        "scan_financial_emails tantivy matching completed"
    );

    let step_started = Instant::now();
    let rows = crate::database::emails::list_email_scan_rows_by_document_ids(
        db.async_connection.clone(),
        &matched_document_ids,
        request.credential_id,
        request.max_emails,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    info!(
        duration_ms = step_started.elapsed().as_millis(),
        rows = rows.len(),
        "scan_financial_emails load_rows completed"
    );

    let step_started = Instant::now();
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
    info!(
        duration_ms = step_started.elapsed().as_millis(),
        unique_senders = sender_counts.len(),
        "scan_financial_emails sender tokenization completed"
    );

    // If a sender's matched emails differ too much at a word level, they are likely not
    // templated transaction emails. We drop them for now (may miss manual emails).
    let step_started = Instant::now();
    let sender_entries: Vec<(String, i64, Vec<Vec<String>>)> = sender_counts
        .into_iter()
        .map(|(sender_email, matched_count)| {
            let tokens = sender_tokens.remove(&sender_email).unwrap_or_default();
            (sender_email, matched_count, tokens)
        })
        .collect();

    let mut senders: Vec<FinancialEmailScanSender> = sender_entries
        .into_par_iter()
        .filter_map(|(sender_email, matched_count, tokens)| {
            let keep = if tokens.len() < 2 {
                true
            } else {
                average_normalized_distance(&tokens) <= TEMPLATE_SIMILARITY_THRESHOLD
            };

            if keep {
                Some(FinancialEmailScanSender {
                    sender_email,
                    matched_count,
                })
            } else {
                None
            }
        })
        .collect();

    let total_matched: i64 = senders.iter().map(|s| s.matched_count).sum();
    info!(
        duration_ms = step_started.elapsed().as_millis(),
        kept_senders = senders.len(),
        total_matched,
        "scan_financial_emails template similarity filter completed"
    );

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

    info!(
        duration_ms = scan_started.elapsed().as_millis(),
        total_emails_scanned = rows.len(),
        total_matched,
        returned_senders = senders.len(),
        "scan_financial_emails completed"
    );

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

    for sample in &samples[1..] {
        let dist = normalized_distance_words(baseline, sample);
        total += dist;
        count += 1.0;
    }

    if count == 0.0 {
        0.0
    } else {
        total / count
    }
}

fn normalized_distance_words(a: &[String], b: &[String]) -> f32 {
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 0.0;
    }
    let dist = levenshtein_words(a, b) as f32;
    dist / max_len as f32
}

fn levenshtein_words(a: &[String], b: &[String]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];

    for (i, aw) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, bw) in b.iter().enumerate() {
            let cost = if aw == bw { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}
