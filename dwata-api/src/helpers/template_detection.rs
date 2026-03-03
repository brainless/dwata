use crate::config::ApiConfig;
use crate::database::{emails as emails_db, financial_templates as templates_db, Database};
use crate::search::tantivy::TantivySearchIndex;
use actix_web::web;
use anyhow::Result;
use dwata_agents::{
    detect_reverse_templates_for_sender, TemplateDetectionOptions, TemplateInputEmail,
};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use regex::Regex;
use shared_types::{
    DataSourceType, DetectFinancialTemplatesRequest, DetectFinancialTemplatesResponse,
    DetectedFinancialTemplate, DetectedFinancialTemplateVariable, DocumentKind,
    FinancialTemplateType, SearchDocumentsRequest, SearchField, SearchTerm,
    TemplateDetectionCandidateEmailPreview, TemplateDetectionDebugState,
    TemplateDetectionGeneratedTemplateDebug, TemplateDetectionSenderDebug,
    TemplateDetectionSenderRank,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

const DEFAULT_FINANCIAL_KEYWORDS: &[&str] = &[
    "payment",
    "paid",
    "debited",
    "credited",
    "invoice",
    "bill",
    "due",
    "transaction",
    "receipt",
    "refunded",
    "bank",
    "statement",
];

const DEFAULT_TEMPLATE_MATCH_THRESHOLD: f64 = 0.55;
const RECENT_EMAIL_WINDOW_MS: i64 = 30_i64 * 24 * 60 * 60 * 1000;
const BILL_TARGET_FIELDS: &[&str] = &[
    "total-amount",
    "currency",
    "issued-date",
    "due-date",
    "billing-period-start",
    "billing-period-end",
    "document-reference",
    "service-identifier",
];
const TRANSACTION_TARGET_FIELDS: &[&str] = &[
    "amount",
    "currency",
    "transaction-date",
    "vendor",
    "transaction-reference",
];

#[derive(Debug, Clone)]
struct TemplateMatchResult {
    template_id: i64,
    score: f64,
}

#[derive(Debug, Clone)]
struct PreparedDetectedCluster {
    template_type: FinancialTemplateType,
    template_body: String,
    translated_template_body: String,
    source_email_ids: Vec<i64>,
    variables: Vec<DetectedFinancialTemplateVariable>,
    has_bill: bool,
}

#[derive(Debug, Clone)]
pub struct TemplateDetectionProgress {
    pub candidate_sender_count: usize,
    pub candidate_email_count: usize,
    pub total_senders: usize,
    pub processed_senders: usize,
    pub current_sender: Option<String>,
    pub debug: Option<TemplateDetectionDebugState>,
}

#[derive(Debug, Clone)]
struct RankedSender {
    sender_email: String,
    total_candidate_emails: usize,
    recent_candidate_emails: usize,
    latest_email_ts: i64,
    max_existing_cluster_size: usize,
}

/// Rank candidate senders for template generation.
///
/// Important: template detection currently executes per sender, but we want to
/// prioritize likely template-heavy clusters first. We use sender-level stats as
/// a proxy for cluster quality and keep this policy isolated because ranking is
/// expected to evolve often.
///
/// Current ranking order:
/// 1) largest known template cluster size for sender (desc)
/// 2) number of recent candidate emails for sender (desc)
/// 3) recency of latest candidate email for sender (desc)
/// 4) total candidate email count for sender (desc)
/// 5) sender email lexical tie-breaker (asc, deterministic)
fn rank_candidate_senders(
    scan_rows: &[emails_db::EmailScanRow],
    sender_cluster_sizes: &HashMap<String, usize>,
) -> Vec<RankedSender> {
    if scan_rows.is_empty() {
        return Vec::new();
    }

    // "Recent" is defined relative to the freshest candidate email seen in this
    // detection run, not wall-clock time, so offline/backfill datasets rank well.
    let max_seen_ts = scan_rows
        .iter()
        .map(|row| row.date_received)
        .max()
        .unwrap_or(0);
    let recent_cutoff = max_seen_ts.saturating_sub(RECENT_EMAIL_WINDOW_MS);

    let mut sender_stats: HashMap<String, (usize, usize, i64)> = HashMap::new();
    for row in scan_rows {
        let sender_key = row.from_address.trim().to_ascii_lowercase();
        if sender_key.is_empty() {
            continue;
        }
        let entry = sender_stats
            .entry(sender_key)
            .or_insert((0usize, 0usize, row.date_received));
        entry.0 += 1;
        if row.date_received >= recent_cutoff {
            entry.1 += 1;
        }
        if row.date_received > entry.2 {
            entry.2 = row.date_received;
        }
    }

    let mut ranked = sender_stats
        .into_iter()
        .map(
            |(sender_email, (total_candidate_emails, recent_candidate_emails, latest_email_ts))| {
                RankedSender {
                    max_existing_cluster_size: sender_cluster_sizes
                        .get(&sender_email)
                        .copied()
                        .unwrap_or(0),
                    sender_email,
                    total_candidate_emails,
                    recent_candidate_emails,
                    latest_email_ts,
                }
            },
        )
        .collect::<Vec<_>>();

    ranked.sort_by(|a, b| {
        b.max_existing_cluster_size
            .cmp(&a.max_existing_cluster_size)
            .then_with(|| b.recent_candidate_emails.cmp(&a.recent_candidate_emails))
            .then_with(|| b.latest_email_ts.cmp(&a.latest_email_ts))
            .then_with(|| b.total_candidate_emails.cmp(&a.total_candidate_emails))
            .then_with(|| a.sender_email.cmp(&b.sender_email))
    });

    ranked
}

fn placeholder_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{\s*placeholder_[^}]+\}\}").expect("valid regex"))
}

fn template_variable_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{\s*([a-zA-Z0-9_-]+)\s*\}\}").expect("valid regex"))
}

fn collect_template_variable_names(template_body: &str) -> HashSet<String> {
    template_variable_regex()
        .captures_iter(template_body)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn is_allowed_target_field(template_type: FinancialTemplateType, field: &str) -> bool {
    match template_type {
        FinancialTemplateType::Bill => BILL_TARGET_FIELDS.contains(&field),
        FinancialTemplateType::Transaction => TRANSACTION_TARGET_FIELDS.contains(&field),
    }
}

fn sanitize_cluster_variables(
    template_body: &str,
    template_type: FinancialTemplateType,
    variables: &[dwata_agents::TemplateVariableMapping],
) -> Vec<DetectedFinancialTemplateVariable> {
    let template_placeholders = collect_template_variable_names(template_body);
    let mut seen_placeholders = HashSet::new();
    let mut out = Vec::new();

    for v in variables {
        let placeholder_name = v.placeholder_name.trim();
        let target_field = v.target_field.trim();
        if placeholder_name.is_empty() || target_field.is_empty() {
            continue;
        }
        if !template_placeholders.contains(placeholder_name) {
            continue;
        }
        if !is_allowed_target_field(template_type, target_field) {
            continue;
        }
        if !seen_placeholders.insert(placeholder_name.to_string()) {
            continue;
        }
        out.push(DetectedFinancialTemplateVariable {
            placeholder_name: placeholder_name.to_string(),
            target_field: target_field.to_string(),
        });
    }

    out
}

fn has_total_amount_mapping(variables: &[DetectedFinancialTemplateVariable]) -> bool {
    variables.iter().any(|v| v.target_field == "total-amount")
}

fn render_translated_template(
    template_body: &str,
    variables: &[DetectedFinancialTemplateVariable],
) -> String {
    let mut out = template_body.to_string();
    for v in variables {
        let old = format!("{{{{ {} }}}}", v.placeholder_name);
        let new = format!("{{{{ {} }}}}", v.target_field);
        out = out.replace(&old, &new);
    }
    out
}

fn split_fixed_segments(template_body: &str) -> Vec<&str> {
    placeholder_regex()
        .split(template_body)
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn match_template_score(template_body: &str, email_text: &str) -> f64 {
    let segments = split_fixed_segments(template_body);
    if segments.is_empty() {
        return 0.0;
    }
    let total_fixed_chars: usize = segments.iter().map(|s| s.len()).sum();
    if total_fixed_chars == 0 {
        return 0.0;
    }

    let mut cursor = 0usize;
    let mut matched_chars = 0usize;
    for segment in &segments {
        if let Some(rel_pos) = email_text[cursor..].find(segment) {
            matched_chars += segment.len();
            cursor += rel_pos + segment.len();
        } else {
            return 0.0;
        }
    }

    let coverage = matched_chars as f64 / total_fixed_chars as f64;
    let order_bonus = matched_chars as f64 / email_text.len().max(1) as f64;
    (coverage * 0.85 + order_bonus * 0.15).clamp(0.0, 1.0)
}

fn match_email_to_sender_templates(
    email_text: &str,
    templates: &[templates_db::SenderFinancialTemplateRow],
    score_threshold: f64,
) -> Option<TemplateMatchResult> {
    let mut best: Option<TemplateMatchResult> = None;
    for template in templates {
        let score = match_template_score(&template.template_body, email_text);
        if score >= score_threshold {
            match &best {
                Some(current) if score <= current.score => {}
                _ => {
                    best = Some(TemplateMatchResult {
                        template_id: template.template_id,
                        score,
                    })
                }
            }
        }
    }
    best
}

fn build_tantivy_query(keywords: &[&str]) -> String {
    keywords
        .iter()
        .map(|keyword| keyword.trim())
        .filter(|keyword| !keyword.is_empty())
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn to_template_type(has_bill: bool) -> FinancialTemplateType {
    if has_bill {
        FinancialTemplateType::Bill
    } else {
        FinancialTemplateType::Transaction
    }
}

fn build_llm_client(_config: &ApiConfig) -> Result<Arc<dyn LlmClient>> {
    Ok(Arc::new(OllamaClient::new()?))
}

fn to_input_email(row: &emails_db::TemplateCandidateEmailRow) -> TemplateInputEmail {
    TemplateInputEmail {
        id: row.email_id,
        subject: row.subject.clone().unwrap_or_default(),
        body: row
            .body_text
            .clone()
            .or_else(|| row.body_html.clone())
            .unwrap_or_default(),
    }
}

fn to_matchable_email_text(row: &emails_db::TemplateCandidateEmailRow) -> String {
    format!(
        "Subject: {}\n---\n{}",
        row.subject.clone().unwrap_or_default(),
        row.body_text
            .clone()
            .or_else(|| row.body_html.clone())
            .unwrap_or_default()
    )
}

fn truncate_preview(input: &str, max_chars: usize) -> String {
    let trimmed = input.trim();
    let mut out = String::new();
    for ch in trimmed.chars().take(max_chars) {
        out.push(ch);
    }
    if trimmed.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

async fn link_matching_emails_for_sender(
    db: web::Data<Arc<Database>>,
    sender_templates: &[templates_db::SenderFinancialTemplateRow],
    sender_candidate_rows: &[emails_db::TemplateCandidateEmailRow],
) -> Result<HashSet<i64>> {
    let mut matched_email_ids = HashSet::new();

    for row in sender_candidate_rows {
        let email_text = to_matchable_email_text(row);
        if let Some(best) = match_email_to_sender_templates(
            &email_text,
            sender_templates,
            DEFAULT_TEMPLATE_MATCH_THRESHOLD,
        ) {
            templates_db::insert_template_email_link(
                db.async_connection.clone(),
                best.template_id,
                row.email_id,
                Some(best.score),
            )
            .await?;
            matched_email_ids.insert(row.email_id);
        }
    }

    Ok(matched_email_ids)
}

pub async fn detect_and_store_templates(
    db: web::Data<Arc<Database>>,
    search_index: web::Data<Arc<TantivySearchIndex>>,
    config: web::Data<Arc<ApiConfig>>,
    request: DetectFinancialTemplatesRequest,
) -> Result<DetectFinancialTemplatesResponse> {
    detect_and_store_templates_with_progress(db, search_index, config, request, |_| {}).await
}

pub async fn detect_and_store_templates_with_progress<F>(
    db: web::Data<Arc<Database>>,
    search_index: web::Data<Arc<TantivySearchIndex>>,
    config: web::Data<Arc<ApiConfig>>,
    request: DetectFinancialTemplatesRequest,
    mut on_progress: F,
) -> Result<DetectFinancialTemplatesResponse>
where
    F: FnMut(TemplateDetectionProgress),
{
    let query = build_tantivy_query(DEFAULT_FINANCIAL_KEYWORDS);
    let keyword_list = DEFAULT_FINANCIAL_KEYWORDS
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    let max_candidate_emails = request.max_candidate_emails;

    let mut matched_document_ids = Vec::new();
    let mut seen_document_ids = HashSet::new();
    let mut offset = 0usize;

    loop {
        let page_limit = if let Some(max) = max_candidate_emails {
            let remaining = max.saturating_sub(matched_document_ids.len());
            let page_limit = remaining.min(100);
            if page_limit == 0 {
                break;
            }
            page_limit
        } else {
            100
        };
        let search_result = search_index.search(&SearchDocumentsRequest {
            terms: vec![SearchTerm {
                field: SearchField::Any,
                value: query.clone(),
                is_phrase: false,
            }],
            kind: Some(DocumentKind::Email),
            source_id: None,
            credential_id: request.credential_id,
            limit: Some(page_limit),
            offset: Some(offset),
        })?;

        if search_result.hits.is_empty() {
            break;
        }
        for hit in &search_result.hits {
            if seen_document_ids.insert(hit.document_id) {
                matched_document_ids.push(hit.document_id);
            }
        }
        let fetched = search_result.hits.len();
        if fetched < page_limit {
            break;
        }
        offset += fetched;
    }

    let scan_rows = emails_db::list_email_scan_rows_by_document_ids(
        db.async_connection.clone(),
        &matched_document_ids,
        request.credential_id,
        request.max_candidate_emails,
    )
    .await?;

    let mut sender_emails = scan_rows
        .iter()
        .map(|row| row.from_address.trim().to_ascii_lowercase())
        .filter(|sender| !sender.is_empty())
        .collect::<Vec<_>>();
    sender_emails.sort();
    sender_emails.dedup();
    let sender_cluster_sizes = templates_db::list_sender_max_cluster_sizes(
        db.async_connection.clone(),
        &sender_emails,
        request.credential_id,
    )
    .await?;

    let ranked_senders = rank_candidate_senders(&scan_rows, &sender_cluster_sizes);
    let ranked_senders = ranked_senders
        .into_iter()
        .filter(|s| s.total_candidate_emails >= 2)
        .collect::<Vec<_>>();
    let sender_ranking = ranked_senders
        .iter()
        .enumerate()
        .map(|(idx, s)| TemplateDetectionSenderRank {
            sender_email: s.sender_email.clone(),
            rank: idx + 1,
            total_candidate_emails: s.total_candidate_emails,
            recent_candidate_emails: s.recent_candidate_emails,
            latest_email_ts: s.latest_email_ts,
            max_existing_cluster_size: s.max_existing_cluster_size,
        })
        .collect::<Vec<_>>();
    let candidate_email_previews = scan_rows
        .iter()
        .take(300)
        .map(|row| TemplateDetectionCandidateEmailPreview {
            sender_email: row.from_address.trim().to_ascii_lowercase(),
            date_received: row.date_received,
            subject: row.subject.clone().unwrap_or_default(),
            body_preview: truncate_preview(
                &row.body_text
                    .clone()
                    .or_else(|| row.body_html.clone())
                    .unwrap_or_default(),
                240,
            ),
        })
        .collect::<Vec<_>>();

    let mut debug_state = TemplateDetectionDebugState {
        keyword_query: query.clone(),
        keyword_list,
        max_candidate_emails: max_candidate_emails.unwrap_or(0),
        matched_document_ids_count: matched_document_ids.len(),
        sender_ranking,
        candidate_email_previews,
        sender_debug: Vec::new(),
    };

    let total_senders = ranked_senders.len();
    on_progress(TemplateDetectionProgress {
        candidate_sender_count: total_senders,
        candidate_email_count: scan_rows.len(),
        total_senders,
        processed_senders: 0,
        current_sender: None,
        debug: Some(debug_state.clone()),
    });

    let llm_client = build_llm_client(&config)?;
    let model = MINISTRAL_3_3B_ID.to_string();
    let mut templates = Vec::new();
    for (sender_idx, ranked_sender) in ranked_senders.iter().enumerate() {
        let sender_email = &ranked_sender.sender_email;
        on_progress(TemplateDetectionProgress {
            candidate_sender_count: total_senders,
            candidate_email_count: scan_rows.len(),
            total_senders,
            processed_senders: sender_idx,
            current_sender: Some(sender_email.clone()),
            debug: Some(debug_state.clone()),
        });

        let mut sender_debug = TemplateDetectionSenderDebug {
            sender_email: sender_email.clone(),
            rank: sender_idx + 1,
            sender_candidate_count: 0,
            existing_template_count: 0,
            initially_matched_count: 0,
            fresh_unmatched_count: 0,
            pool_count: 0,
            generated_templates: Vec::new(),
            error: None,
            skipped_reason: None,
        };

        let sender_rows =
            match emails_db::list_template_candidate_emails_by_sender_and_document_ids(
                db.async_connection.clone(),
                sender_email,
                &matched_document_ids,
                request.credential_id,
                request.max_candidate_emails,
            )
            .await
            {
                Ok(rows) => rows,
                Err(err) => {
                    sender_debug.error = Some(err.to_string());
                    debug_state.sender_debug.push(sender_debug);
                    continue;
                }
            };
        sender_debug.sender_candidate_count = sender_rows.len();
        if sender_rows.is_empty() {
            sender_debug.skipped_reason = Some("No sender rows".to_string());
            debug_state.sender_debug.push(sender_debug);
            continue;
        }

        let existing_templates = match templates_db::list_templates_with_variables_by_sender(
            db.async_connection.clone(),
            sender_email,
        )
        .await
        {
            Ok(rows) => rows,
            Err(err) => {
                sender_debug.error = Some(err.to_string());
                debug_state.sender_debug.push(sender_debug);
                continue;
            }
        };
        sender_debug.existing_template_count = existing_templates.len();

        let initially_matched =
            match link_matching_emails_for_sender(db.clone(), &existing_templates, &sender_rows)
                .await
            {
                Ok(ids) => ids,
                Err(err) => {
                    sender_debug.error = Some(err.to_string());
                    debug_state.sender_debug.push(sender_debug);
                    continue;
                }
            };
        sender_debug.initially_matched_count = initially_matched.len();

        let fresh_unmatched_rows =
            match emails_db::list_unmatched_template_candidate_emails_by_sender_and_document_ids(
                db.async_connection.clone(),
                sender_email,
                &matched_document_ids,
                request.credential_id,
                request.max_candidate_emails,
            )
            .await
            {
                Ok(rows) => rows,
                Err(err) => {
                    sender_debug.error = Some(err.to_string());
                    debug_state.sender_debug.push(sender_debug);
                    continue;
                }
            };
        sender_debug.fresh_unmatched_count = fresh_unmatched_rows.len();
        if fresh_unmatched_rows.is_empty() {
            sender_debug.skipped_reason = Some("No unmatched rows".to_string());
            debug_state.sender_debug.push(sender_debug);
            continue;
        }

        let template_ids = existing_templates
            .iter()
            .map(|t| t.template_id)
            .collect::<Vec<_>>();
        let anchor_email_ids = match templates_db::list_anchor_email_ids_by_template_ids(
            db.async_connection.clone(),
            &template_ids,
            request.credential_id,
        )
        .await
        {
            Ok(ids) => ids,
            Err(err) => {
                sender_debug.error = Some(err.to_string());
                debug_state.sender_debug.push(sender_debug);
                continue;
            }
        };

        let sender_rows_by_id = sender_rows
            .iter()
            .map(|r| (r.email_id, r.clone()))
            .collect::<HashMap<_, _>>();

        let mut pool_rows = fresh_unmatched_rows.clone();
        let mut seen_pool_email_ids = pool_rows.iter().map(|r| r.email_id).collect::<HashSet<_>>();
        for template_id in template_ids {
            if let Some(anchor_email_id) = anchor_email_ids.get(&template_id) {
                if seen_pool_email_ids.contains(anchor_email_id) {
                    continue;
                }
                if let Some(anchor_row) = sender_rows_by_id.get(anchor_email_id) {
                    seen_pool_email_ids.insert(*anchor_email_id);
                    pool_rows.push(anchor_row.clone());
                }
            }
        }
        sender_debug.pool_count = pool_rows.len();
        if pool_rows.len() < 2 {
            sender_debug.skipped_reason = Some("Pool smaller than 2 emails".to_string());
            debug_state.sender_debug.push(sender_debug);
            continue;
        }

        let input_emails = pool_rows.iter().map(to_input_email).collect::<Vec<_>>();
        let clusters = match detect_reverse_templates_for_sender(
            llm_client.clone(),
            model.clone(),
            input_emails,
            TemplateDetectionOptions {
                word_distance_threshold: 0.35,
                max_clusters: request.max_templates_per_sender.unwrap_or(3),
            },
        )
        .await
        {
            Ok(c) => c,
            Err(err) => {
                sender_debug.error = Some(err.to_string());
                debug_state.sender_debug.push(sender_debug);
                continue;
            }
        };

        let mut valid_clusters: Vec<PreparedDetectedCluster> = Vec::new();
        for cluster in clusters {
            let template_type = to_template_type(cluster.has_bill);
            let sanitized_variables = sanitize_cluster_variables(
                &cluster.template_body,
                template_type,
                &cluster.variables,
            );

            if sanitized_variables.is_empty() {
                sender_debug
                    .generated_templates
                    .push(TemplateDetectionGeneratedTemplateDebug {
                        template_id: None,
                        template_type: Some(template_type),
                        template_body: cluster.template_body,
                        translated_template_body: cluster.translated_template_body,
                        source_email_ids: cluster.email_ids,
                        variables: Vec::new(),
                        has_bill: cluster.has_bill,
                        discarded_reason: Some(
                            "No valid variable mappings after placeholder validation".to_string(),
                        ),
                    });
                continue;
            }
            if cluster.has_bill && !has_total_amount_mapping(&sanitized_variables) {
                let translated_template_body =
                    render_translated_template(&cluster.template_body, &sanitized_variables);
                sender_debug
                    .generated_templates
                    .push(TemplateDetectionGeneratedTemplateDebug {
                        template_id: None,
                        template_type: Some(template_type),
                        template_body: cluster.template_body,
                        translated_template_body,
                        source_email_ids: cluster.email_ids,
                        variables: sanitized_variables,
                        has_bill: cluster.has_bill,
                        discarded_reason: Some(
                            "Bill template missing total-amount mapping".to_string(),
                        ),
                    });
                continue;
            }
            valid_clusters.push(PreparedDetectedCluster {
                template_type,
                template_body: cluster.template_body.clone(),
                translated_template_body: render_translated_template(
                    &cluster.template_body,
                    &sanitized_variables,
                ),
                source_email_ids: cluster.email_ids.clone(),
                variables: sanitized_variables,
                has_bill: cluster.has_bill,
            });
        }

        if valid_clusters.is_empty() {
            sender_debug.skipped_reason = Some("No valid clusters after validation".to_string());
            debug_state.sender_debug.push(sender_debug);
            continue;
        }

        for cluster in valid_clusters {
            let template_type = cluster.template_type;
            let template_id = match templates_db::insert_template(
                db.async_connection.clone(),
                DataSourceType::Email,
                sender_email,
                template_type,
                &cluster.template_body,
            )
            .await
            {
                Ok(id) => id,
                Err(err) => {
                    sender_debug.error = Some(err.to_string());
                    break;
                }
            };

            if let Err(err) = templates_db::insert_template_applicability(
                db.async_connection.clone(),
                template_id,
                DataSourceType::Email,
                sender_email,
                None,
            )
            .await
            {
                sender_debug.error = Some(err.to_string());
                break;
            }

            for variable in &cluster.variables {
                if let Err(err) = templates_db::insert_template_variable(
                    db.async_connection.clone(),
                    template_id,
                    &variable.placeholder_name,
                    &variable.target_field,
                )
                .await
                {
                    sender_debug.error = Some(err.to_string());
                    break;
                }
            }
            if sender_debug.error.is_some() {
                break;
            }

            for email_id in &cluster.source_email_ids {
                if let Err(err) = templates_db::insert_template_email_link(
                    db.async_connection.clone(),
                    template_id,
                    *email_id,
                    None,
                )
                .await
                {
                    sender_debug.error = Some(err.to_string());
                    break;
                }
            }
            if sender_debug.error.is_some() {
                break;
            }

            templates.push(DetectedFinancialTemplate {
                template_id,
                sender_email: sender_email.clone(),
                template_type,
                template_body: cluster.template_body.clone(),
                translated_template_body: cluster.translated_template_body.clone(),
                source_email_ids: cluster.source_email_ids.clone(),
                variables: cluster.variables.clone(),
            });

            sender_debug
                .generated_templates
                .push(TemplateDetectionGeneratedTemplateDebug {
                    template_id: Some(template_id),
                    template_type: Some(template_type),
                    template_body: cluster.template_body,
                    translated_template_body: cluster.translated_template_body,
                    source_email_ids: cluster.source_email_ids,
                    variables: cluster.variables,
                    has_bill: cluster.has_bill,
                    discarded_reason: None,
                });
        }

        let refreshed_templates = match templates_db::list_templates_with_variables_by_sender(
            db.async_connection.clone(),
            sender_email,
        )
        .await
        {
            Ok(rows) => rows,
            Err(err) => {
                sender_debug.error = Some(err.to_string());
                debug_state.sender_debug.push(sender_debug);
                continue;
            }
        };
        if let Err(err) =
            link_matching_emails_for_sender(db.clone(), &refreshed_templates, &sender_rows).await
        {
            sender_debug.error = Some(err.to_string());
        }
        debug_state.sender_debug.push(sender_debug);

        on_progress(TemplateDetectionProgress {
            candidate_sender_count: total_senders,
            candidate_email_count: scan_rows.len(),
            total_senders,
            processed_senders: sender_idx + 1,
            current_sender: None,
            debug: Some(debug_state.clone()),
        });
    }

    Ok(DetectFinancialTemplatesResponse {
        candidate_sender_count: total_senders,
        candidate_email_count: scan_rows.len(),
        templates,
    })
}

#[cfg(test)]
mod tests {
    use super::{match_email_to_sender_templates, match_template_score, rank_candidate_senders};
    use crate::database::emails::EmailScanRow;
    use crate::database::financial_templates::SenderFinancialTemplateRow;
    use shared_types::FinancialTemplateType;
    use std::collections::HashMap;

    #[test]
    fn template_score_matches_placeholder_wildcards() {
        let template = "Subject: Payment received {{ placeholder_amount }}\n---\nRef {{ placeholder_ref }} from {{ placeholder_sender }}";
        let email = "Subject: Payment received $49.99\n---\nRef TX-123 from ACME Corp";
        let score = match_template_score(template, email);
        assert!(score > 0.55);
    }

    #[test]
    fn template_score_requires_all_fixed_segments_in_order() {
        let template =
            "Subject: Invoice {{ placeholder_id }}\n---\nTotal {{ placeholder_total }} due";
        let wrong_order = "Subject: Invoice 100\n---\ndue and then Total $12";
        let score = match_template_score(template, wrong_order);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn best_template_is_selected_by_score() {
        let email = "Subject: Payment received $49.99\n---\nRef TX-123 from ACME Corp";
        let templates = vec![
            SenderFinancialTemplateRow {
                template_id: 1,
                template_type: FinancialTemplateType::Transaction,
                template_body: "Subject: Invoice {{ placeholder_id }}".to_string(),
                variables: Vec::new(),
            },
            SenderFinancialTemplateRow {
                template_id: 2,
                template_type: FinancialTemplateType::Transaction,
                template_body: "Subject: Payment received {{ placeholder_amount }}\n---\nRef {{ placeholder_ref }} from {{ placeholder_sender }}".to_string(),
                variables: Vec::new(),
            },
        ];

        let matched = match_email_to_sender_templates(email, &templates, 0.2).expect("match");
        assert_eq!(matched.template_id, 2);
    }

    #[test]
    fn ranking_prioritizes_cluster_size_then_recent_candidate_count() {
        let rows = vec![
            EmailScanRow {
                from_address: "a@example.com".to_string(),
                date_received: 1_000_000,
                subject: None,
                body_text: None,
                body_html: None,
            },
            EmailScanRow {
                from_address: "a@example.com".to_string(),
                date_received: 999_900,
                subject: None,
                body_text: None,
                body_html: None,
            },
            EmailScanRow {
                from_address: "b@example.com".to_string(),
                date_received: 1_000_000,
                subject: None,
                body_text: None,
                body_html: None,
            },
            EmailScanRow {
                from_address: "c@example.com".to_string(),
                date_received: 1_000_000,
                subject: None,
                body_text: None,
                body_html: None,
            },
            EmailScanRow {
                from_address: "c@example.com".to_string(),
                date_received: 999_950,
                subject: None,
                body_text: None,
                body_html: None,
            },
            EmailScanRow {
                from_address: "c@example.com".to_string(),
                date_received: 999_940,
                subject: None,
                body_text: None,
                body_html: None,
            },
        ];

        let mut cluster_sizes = HashMap::new();
        cluster_sizes.insert("a@example.com".to_string(), 5);
        cluster_sizes.insert("b@example.com".to_string(), 7);
        cluster_sizes.insert("c@example.com".to_string(), 5);

        let ranked = rank_candidate_senders(&rows, &cluster_sizes);
        let ordered = ranked
            .into_iter()
            .map(|r| r.sender_email)
            .collect::<Vec<_>>();

        assert_eq!(
            ordered,
            vec![
                "b@example.com".to_string(),
                "c@example.com".to_string(),
                "a@example.com".to_string()
            ]
        );
    }
}
