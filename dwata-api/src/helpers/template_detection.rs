use crate::config::ApiConfig;
use crate::database::{emails as emails_db, financial_templates as templates_db, Database};
use crate::search::tantivy::TantivySearchIndex;
use actix_web::web;
use anyhow::Result;
use dwata_agents::{detect_templates_for_sender, TemplateDetectionOptions, TemplateInputEmail};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::GEMINI_3_FLASH_ID;
use regex::Regex;
use shared_types::{
    DataSourceType, DetectFinancialTemplatesRequest, DetectFinancialTemplatesResponse,
    DetectedFinancialTemplate, DetectedFinancialTemplateVariable, DocumentKind,
    FinancialTemplateType, SearchDocumentsRequest, SearchField, SearchTerm,
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

#[derive(Debug, Clone)]
struct TemplateMatchResult {
    template_id: i64,
    score: f64,
}

#[derive(Debug, Clone)]
pub struct TemplateDetectionProgress {
    pub candidate_sender_count: usize,
    pub candidate_email_count: usize,
    pub total_senders: usize,
    pub processed_senders: usize,
    pub current_sender: Option<String>,
}

fn placeholder_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{\s*placeholder_[^}]+\}\}").expect("valid regex"))
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

fn build_llm_client(config: &ApiConfig) -> Result<Arc<dyn LlmClient>> {
    let api_key = config
        .ai_provider_api_keys
        .as_ref()
        .and_then(|keys| keys.gemini_api_key.as_ref())
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!("Missing ai_provider_api_keys.gemini_api_key in api config")
        })?;
    Ok(Arc::new(GeminiClient::new(api_key)?))
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
    let max_candidate_emails = request.max_candidate_emails.unwrap_or(2000);

    let mut matched_document_ids = Vec::new();
    let mut seen_document_ids = HashSet::new();
    let mut offset = 0usize;

    while matched_document_ids.len() < max_candidate_emails {
        let remaining = max_candidate_emails.saturating_sub(matched_document_ids.len());
        let page_limit = remaining.min(100);
        if page_limit == 0 {
            break;
        }
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

    let mut sender_counts: HashMap<String, usize> = HashMap::new();
    for row in &scan_rows {
        *sender_counts.entry(row.from_address.clone()).or_insert(0) += 1;
    }
    let mut senders = sender_counts.into_iter().collect::<Vec<_>>();
    senders.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    if let Some(max_senders) = request.max_senders {
        if senders.len() > max_senders {
            senders.truncate(max_senders);
        }
    }
    let total_senders = senders.len();
    on_progress(TemplateDetectionProgress {
        candidate_sender_count: total_senders,
        candidate_email_count: scan_rows.len(),
        total_senders,
        processed_senders: 0,
        current_sender: None,
    });

    let llm_client = build_llm_client(&config)?;
    let model = GEMINI_3_FLASH_ID.to_string();
    let mut templates = Vec::new();

    for (sender_idx, (sender_email, _)) in senders.iter().enumerate() {
        on_progress(TemplateDetectionProgress {
            candidate_sender_count: total_senders,
            candidate_email_count: scan_rows.len(),
            total_senders,
            processed_senders: sender_idx,
            current_sender: Some(sender_email.clone()),
        });

        let sender_rows = emails_db::list_template_candidate_emails_by_sender_and_document_ids(
            db.async_connection.clone(),
            sender_email,
            &matched_document_ids,
            request.credential_id,
            request.max_candidate_emails,
        )
        .await?;
        if sender_rows.is_empty() {
            continue;
        }

        let existing_templates = templates_db::list_templates_with_variables_by_sender(
            db.async_connection.clone(),
            sender_email,
        )
        .await?;

        let _ =
            link_matching_emails_for_sender(db.clone(), &existing_templates, &sender_rows).await?;

        let fresh_unmatched_rows =
            emails_db::list_unmatched_template_candidate_emails_by_sender_and_document_ids(
                db.async_connection.clone(),
                sender_email,
                &matched_document_ids,
                request.credential_id,
                request.max_candidate_emails,
            )
            .await?;

        if fresh_unmatched_rows.is_empty() {
            continue;
        }

        let template_ids = existing_templates
            .iter()
            .map(|t| t.template_id)
            .collect::<Vec<_>>();
        let anchor_email_ids = templates_db::list_anchor_email_ids_by_template_ids(
            db.async_connection.clone(),
            &template_ids,
            request.credential_id,
        )
        .await?;

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

        if pool_rows.len() < 2 {
            continue;
        }

        let input_emails = pool_rows.iter().map(to_input_email).collect::<Vec<_>>();
        let clusters = detect_templates_for_sender(
            llm_client.clone(),
            model.clone(),
            input_emails,
            TemplateDetectionOptions {
                word_distance_threshold: 0.35,
                max_clusters: request.max_templates_per_sender.unwrap_or(3),
            },
        )
        .await?;

        for cluster in clusters {
            let template_type = to_template_type(cluster.has_bill);
            let template_id = templates_db::insert_template(
                db.async_connection.clone(),
                DataSourceType::Email,
                sender_email,
                template_type,
                &cluster.template_body,
            )
            .await?;

            templates_db::insert_template_applicability(
                db.async_connection.clone(),
                template_id,
                DataSourceType::Email,
                sender_email,
                None,
            )
            .await?;

            for variable in &cluster.variables {
                templates_db::insert_template_variable(
                    db.async_connection.clone(),
                    template_id,
                    &variable.placeholder_name,
                    &variable.target_field,
                )
                .await?;
            }
            for email_id in &cluster.email_ids {
                templates_db::insert_template_email_link(
                    db.async_connection.clone(),
                    template_id,
                    *email_id,
                    None,
                )
                .await?;
            }

            templates.push(DetectedFinancialTemplate {
                template_id,
                sender_email: sender_email.clone(),
                template_type,
                template_body: cluster.template_body,
                translated_template_body: cluster.translated_template_body,
                source_email_ids: cluster.email_ids,
                variables: cluster
                    .variables
                    .into_iter()
                    .map(|v| DetectedFinancialTemplateVariable {
                        placeholder_name: v.placeholder_name,
                        target_field: v.target_field,
                    })
                    .collect(),
            });
        }

        let refreshed_templates = templates_db::list_templates_with_variables_by_sender(
            db.async_connection.clone(),
            sender_email,
        )
        .await?;
        let _ =
            link_matching_emails_for_sender(db.clone(), &refreshed_templates, &sender_rows).await?;

        on_progress(TemplateDetectionProgress {
            candidate_sender_count: total_senders,
            candidate_email_count: scan_rows.len(),
            total_senders,
            processed_senders: sender_idx + 1,
            current_sender: None,
        });
    }

    Ok(DetectFinancialTemplatesResponse {
        candidate_sender_count: senders.len(),
        candidate_email_count: scan_rows.len(),
        templates,
    })
}

#[cfg(test)]
mod tests {
    use super::{match_email_to_sender_templates, match_template_score};
    use crate::database::financial_templates::SenderFinancialTemplateRow;
    use shared_types::FinancialTemplateType;

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
}
