use crate::config::ApiConfig;
use crate::database::{emails as emails_db, financial_templates as templates_db, Database};
use crate::search::tantivy::TantivySearchIndex;
use actix_web::web;
use anyhow::Result;
use dwata_agents::{detect_templates_for_sender, TemplateDetectionOptions, TemplateInputEmail};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::GEMINI_3_FLASH_ID;
use shared_types::{
    DataSourceType, DetectFinancialTemplatesRequest, DetectFinancialTemplatesResponse,
    DetectedFinancialTemplate, DetectedFinancialTemplateVariable, DocumentKind,
    FinancialTemplateType, SearchDocumentsRequest, SearchField, SearchTerm,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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

fn build_tantivy_query(keywords: &[&str]) -> String {
    keywords
        .iter()
        .map(|keyword| format!("\"{}\"", keyword.replace('"', "\\\"")))
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

pub async fn detect_and_store_templates(
    db: web::Data<Arc<Database>>,
    search_index: web::Data<Arc<TantivySearchIndex>>,
    config: web::Data<Arc<ApiConfig>>,
    request: DetectFinancialTemplatesRequest,
) -> Result<DetectFinancialTemplatesResponse> {
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

    let llm_client = build_llm_client(&config)?;
    let model = GEMINI_3_FLASH_ID.to_string();
    let mut templates = Vec::new();
    for (sender_email, _) in &senders {
        let sender_rows = emails_db::list_template_candidate_emails_by_sender_and_document_ids(
            db.async_connection.clone(),
            sender_email,
            &matched_document_ids,
            request.credential_id,
            request.max_candidate_emails,
        )
        .await?;

        if sender_rows.len() < 2 {
            continue;
        }

        let input_emails = sender_rows
            .iter()
            .map(|row| TemplateInputEmail {
                id: row.email_id,
                subject: row.subject.clone().unwrap_or_default(),
                body: row
                    .body_text
                    .clone()
                    .or_else(|| row.body_html.clone())
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();

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
    }

    Ok(DetectFinancialTemplatesResponse {
        candidate_sender_count: senders.len(),
        candidate_email_count: scan_rows.len(),
        templates,
    })
}
