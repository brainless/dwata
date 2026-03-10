use crate::storage::{AgentStorage, InMemoryAgentStorage, Session};
use crate::{
    LlmTemplateVariableExtractorAgent, TemplateDocumentLabelerAgent, TemplateVariable,
    TemplateVariableType,
};
use anyhow::Result;
use nocodo_llm_sdk::client::LlmClient;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReverseTemplateType {
    Bill,
    Transaction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReverseTemplateField {
    TotalAmount,
    Currency,
    IssuedDate,
    DueDate,
    BillingPeriodStart,
    BillingPeriodEnd,
    DocumentReference,
    ServiceIdentifier,
    Category,
    Amount,
    TransactionDate,
    PayerVendorId,
    PayeeVendorId,
    TransactionReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseVariableTranslation {
    pub placeholder_name: String,
    pub target_field: ReverseTemplateField,
}

#[derive(Debug, Clone)]
pub struct TemplateInputEmail {
    pub id: i64,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct NormalizedEmailContent {
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct TemplateDetectionOptions {
    pub word_distance_threshold: f64,
    pub max_clusters: usize,
}

#[derive(Debug, Clone)]
pub struct TemplateVariableMapping {
    pub placeholder_name: String,
    pub target_field: String,
}

#[derive(Debug, Clone)]
pub struct TemplateDraft {
    pub seed_text: String,
    pub cluster_size: usize,
    pub selected_email_ids: Vec<i64>,
    pub full_template: String,
    pub line_support: f64,
    pub word_support: f64,
}

#[derive(Debug, Clone)]
pub struct DetectedTemplateCluster {
    pub template_body: String,
    pub translated_template_body: String,
    pub seed_text: String,
    pub email_ids: Vec<i64>,
    pub variables: Vec<TemplateVariableMapping>,
    pub has_bill: bool,
}

#[derive(Debug, Clone)]
struct TemplateDefaults {
    line_support: f64,
    word_support: f64,
}

#[derive(Debug, Clone)]
struct EmailCluster {
    seed_text: String,
    members: Vec<TemplateInputEmail>,
}

const BILL_FIELDS: &[&str] = &[
    "total-amount",
    "currency",
    "issued-date",
    "due-date",
    "billing-period-start",
    "billing-period-end",
    "document-reference",
    "service-identifier",
];

const TRANSACTION_FIELDS: &[&str] = &[
    "amount",
    "currency",
    "transaction-date",
    "vendor",
    "transaction-reference",
];

/// Build plain-text subject/body from raw email content.
///
/// Prefer `body_text` when present, fallback to HTML→text conversion.
/// Unlike normalize_email_content, this preserves original line structure.
pub fn simple_email_content(
    subject: Option<&str>,
    body_text: Option<&str>,
    body_html: Option<&str>,
) -> NormalizedEmailContent {
    let subject = subject.unwrap_or_default().trim().to_string();
    let body = if let Some(text) = body_text {
        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() {
            trimmed
        } else {
            extract_text_from_html(body_html.unwrap_or_default())
        }
    } else {
        extract_text_from_html(body_html.unwrap_or_default())
    };

    NormalizedEmailContent { subject, body }
}

/// Build normalized plain-text subject/body from raw email content.
///
/// This is the canonical source for template-detection text preparation:
/// - Prefer `body_text` when present.
/// - Fallback to lossy HTML-to-text extraction from `body_html`.
/// - Apply consistent whitespace/newline cleanup for downstream agents.
pub fn normalize_email_content(
    subject: Option<&str>,
    body_text: Option<&str>,
    body_html: Option<&str>,
) -> NormalizedEmailContent {
    let subject = clean_subject_text(subject.unwrap_or_default());
    let body = if let Some(text) = body_text {
        let cleaned = clean_plain_text(text);
        if !cleaned.is_empty() {
            cleaned
        } else {
            clean_plain_text(&extract_text_from_html(body_html.unwrap_or_default()))
        }
    } else {
        clean_plain_text(&extract_text_from_html(body_html.unwrap_or_default()))
    };

    NormalizedEmailContent { subject, body }
}

fn clean_plain_text(raw: &str) -> String {
    let mut out = raw.replace('\r', "").replace('\u{00A0}', " ");
    if let Ok(re_ws) = Regex::new(r"[ \t]+") {
        out = re_ws.replace_all(&out, " ").to_string();
    }
    if let Ok(re_nl) = Regex::new(r"\n{3,}") {
        out = re_nl.replace_all(&out, "\n\n").to_string();
    }
    let mut paragraphs = Vec::new();
    let mut current_lines = Vec::new();

    for line in out.lines().map(|line| line.trim()) {
        if line.is_empty() {
            if !current_lines.is_empty() {
                paragraphs.push(merge_lines_into_sentences(&current_lines));
                current_lines.clear();
            }
            continue;
        }
        current_lines.push(line.to_string());
    }

    if !current_lines.is_empty() {
        paragraphs.push(merge_lines_into_sentences(&current_lines));
    }

    paragraphs
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
        .trim()
        .to_string()
}

fn merge_lines_into_sentences(lines: &[String]) -> String {
    let merged = lines.join(" ");
    split_into_sentences(&merged)
        .into_iter()
        .map(|sentence| sentence.trim().to_string())
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn clean_subject_text(raw: &str) -> String {
    let mut out = raw
        .replace('\r', " ")
        .replace('\n', " ")
        .replace('\u{00A0}', " ");
    if let Ok(re_ws) = Regex::new(r"\s+") {
        out = re_ws.replace_all(&out, " ").to_string();
    }
    out.trim().to_string()
}

fn extract_text_from_html(html: &str) -> String {
    if html.trim().is_empty() {
        return String::new();
    }

    let mut out = html.to_string();
    if let Ok(re_script_style) = Regex::new(r"(?is)<(script|style)[^>]*>.*?</(script|style)>") {
        out = re_script_style.replace_all(&out, " ").to_string();
    }
    if let Ok(re_block_breaks) = Regex::new(r"(?i)<\s*(br|/p|/div|/li|/tr|/h[1-6])\s*/?>") {
        out = re_block_breaks.replace_all(&out, "\n").to_string();
    }
    if let Ok(re_li) = Regex::new(r"(?i)<\s*li[^>]*>") {
        out = re_li.replace_all(&out, "\n- ").to_string();
    }
    if let Ok(re_tags) = Regex::new(r"(?is)<[^>]+>") {
        out = re_tags.replace_all(&out, " ").to_string();
    }

    out = out
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");

    if let Ok(re_numeric_entity) = Regex::new(r"&#([0-9]{1,7});") {
        out = re_numeric_entity
            .replace_all(&out, |caps: &regex::Captures| {
                caps.get(1)
                    .and_then(|m| m.as_str().parse::<u32>().ok())
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_default()
            })
            .to_string();
    }
    if let Ok(re_hex_entity) = Regex::new(r"&#x([0-9a-fA-F]{1,6});") {
        out = re_hex_entity
            .replace_all(&out, |caps: &regex::Captures| {
                caps.get(1)
                    .and_then(|m| u32::from_str_radix(m.as_str(), 16).ok())
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_default()
            })
            .to_string();
    }

    out
}

fn canonical_field_aliases(
    template_type: ReverseTemplateType,
    fields: &[&str],
) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for field in fields {
        let canonical = (*field).to_string();
        aliases.insert(canonical.clone(), canonical.clone());
        aliases.insert(field.replace('-', "_"), canonical);
    }
    if matches!(template_type, ReverseTemplateType::Transaction) {
        aliases.insert("vendor_name".to_string(), "vendor".to_string());
    }
    aliases
}

pub fn discover_template_drafts(
    emails: Vec<TemplateInputEmail>,
    options: TemplateDetectionOptions,
) -> Result<Vec<TemplateDraft>> {
    let mut clusters = cluster_emails(emails, options.word_distance_threshold)?;
    clusters.retain(|c| c.members.len() >= 2);
    if clusters.is_empty() {
        return Ok(Vec::new());
    }
    clusters.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
    if options.max_clusters > 0 && clusters.len() > options.max_clusters {
        clusters.truncate(options.max_clusters);
    }

    let drafts = clusters
        .iter()
        .map(|cluster| {
            let defaults = derive_template_defaults(cluster.members.len());
            let max_template_emails = derive_max_template_emails(cluster.members.len());
            let mut scored: Vec<(f64, &TemplateInputEmail)> = cluster
                .members
                .iter()
                .map(|e| {
                    let dist =
                        normalized_word_edit_distance(&cluster.seed_text, &comparable_text(e));
                    (dist, e)
                })
                .collect();
            scored.sort_by(|a, b| a.0.total_cmp(&b.0));
            let sorted_emails: Vec<&TemplateInputEmail> =
                scored.into_iter().map(|(_, e)| e).collect();
            let selected: Vec<&TemplateInputEmail> = sorted_emails
                .iter()
                .take(max_template_emails)
                .copied()
                .collect();
            let (subject_template, body_template) = build_adaptive_template_from_candidates(
                &sorted_emails,
                max_template_emails,
                &defaults,
            );
            let full_template = format!("Subject: {subject_template}\n---\n{body_template}");

            TemplateDraft {
                seed_text: cluster.seed_text.clone(),
                cluster_size: cluster.members.len(),
                selected_email_ids: selected.iter().map(|e| e.id).collect(),
                full_template,
                line_support: defaults.line_support,
                word_support: defaults.word_support,
            }
        })
        .collect();

    Ok(drafts)
}

pub async fn detect_templates_for_sender(
    llm_client: Arc<dyn LlmClient>,
    model: String,
    emails: Vec<TemplateInputEmail>,
    options: TemplateDetectionOptions,
) -> Result<Vec<DetectedTemplateCluster>> {
    detect_reverse_templates_for_sender(llm_client, model, emails, options).await
}

pub async fn detect_reverse_templates_for_sender(
    llm_client: Arc<dyn LlmClient>,
    model: String,
    emails: Vec<TemplateInputEmail>,
    options: TemplateDetectionOptions,
) -> Result<Vec<DetectedTemplateCluster>> {
    let email_by_id = emails
        .iter()
        .map(|email| (email.id, email.clone()))
        .collect::<HashMap<_, _>>();
    let drafts = discover_template_drafts(emails, options)?;
    let mut out = Vec::new();

    for draft in drafts {
        let storage = Arc::new(InMemoryAgentStorage::new());
        let label = {
            let session_id = storage
                .create_session(Session {
                    id: None,
                    agent_type: "template-document-labeler".to_string(),
                    objective: "Classify financial document type".to_string(),
                    context_data: None,
                    status: "running".to_string(),
                    result: None,
                })
                .await?;
            let agent = TemplateDocumentLabelerAgent::new(
                llm_client.clone(),
                storage.clone(),
                model.clone(),
                draft.full_template.clone(),
            );
            agent.execute(session_id).await?
        };

        let mut template_types = Vec::new();
        if label.has_bill {
            template_types.push(ReverseTemplateType::Bill);
        }
        if label.has_transaction {
            template_types.push(ReverseTemplateType::Transaction);
        }
        if template_types.is_empty() {
            continue;
        }

        let sample_email = match draft
            .selected_email_ids
            .iter()
            .find_map(|id| email_by_id.get(id))
            .cloned()
        {
            Some(sample) => sample,
            None => continue,
        };

        for template_type in template_types {
            let var_type = match template_type {
                ReverseTemplateType::Bill => TemplateVariableType::Bill,
                ReverseTemplateType::Transaction => TemplateVariableType::Transaction,
            };
            let reverse_output = {
                let session_id = storage
                    .create_session(Session {
                        id: None,
                        agent_type: "llm-template-variable-extractor".to_string(),
                        objective: "Extract template variables from email sample".to_string(),
                        context_data: None,
                        status: "running".to_string(),
                        result: None,
                    })
                    .await?;
                let agent = LlmTemplateVariableExtractorAgent::new(
                    llm_client.clone(),
                    storage.clone(),
                    model.clone(),
                    var_type,
                    sample_email.subject.clone(),
                    sample_email.body.clone(),
                );
                agent.execute(session_id).await?
            };

            let field_aliases = match template_type {
                ReverseTemplateType::Bill => canonical_field_aliases(template_type, BILL_FIELDS),
                ReverseTemplateType::Transaction => {
                    canonical_field_aliases(template_type, TRANSACTION_FIELDS)
                }
            };

            let variables: Vec<TemplateVariableMapping> = reverse_output
                .variables
                .iter()
                .filter_map(|v| {
                    let target_field = field_aliases.get(&v.variable_name)?.clone();
                    Some(TemplateVariableMapping {
                        placeholder_name: v.variable_name.clone(),
                        target_field,
                    })
                })
                .collect();

            let template_body = reconstruct_template_from_variables(
                &sample_email.subject,
                &sample_email.body,
                &reverse_output.variables,
            );

            if matches!(template_type, ReverseTemplateType::Bill)
                && !variables.iter().any(|v| v.target_field == "total-amount")
            {
                tracing::warn!("Discarding reverse bill template without total-amount placeholder");
                continue;
            }

            out.push(DetectedTemplateCluster {
                template_body: template_body.clone(),
                translated_template_body: template_body,
                seed_text: draft.seed_text.clone(),
                email_ids: draft.selected_email_ids.clone(),
                variables,
                has_bill: matches!(template_type, ReverseTemplateType::Bill),
            });
        }
    }

    Ok(out)
}

fn cluster_emails(
    candidates: Vec<TemplateInputEmail>,
    threshold: f64,
) -> Result<Vec<EmailCluster>> {
    if !(0.0..=1.0).contains(&threshold) {
        return Err(anyhow::anyhow!(
            "--word-distance-threshold must be between 0.0 and 1.0"
        ));
    }

    let total_count = candidates.len();
    if total_count < 2 {
        return Ok(Vec::new());
    }

    let mut clusters: Vec<EmailCluster> = Vec::new();
    let early_abort_at = (total_count / 3).max(2);

    for (idx, email) in candidates.into_iter().enumerate() {
        let email_text = comparable_text(&email);
        let best = clusters
            .iter()
            .enumerate()
            .map(|(ci, c)| (ci, normalized_word_edit_distance(&c.seed_text, &email_text)))
            .min_by(|a, b| a.1.total_cmp(&b.1));

        match best {
            Some((best_ci, best_dist)) if best_dist <= threshold => {
                clusters[best_ci].members.push(email);
            }
            _ => {
                clusters.push(EmailCluster {
                    seed_text: email_text,
                    members: vec![email],
                });
            }
        }

        if idx + 1 == early_abort_at {
            let all_singletons = clusters.iter().all(|c| c.members.len() == 1);
            if all_singletons {
                return Ok(Vec::new());
            }
        }
    }

    Ok(clusters)
}

fn derive_max_template_emails(matching_count: usize) -> usize {
    if matching_count >= 30 {
        24
    } else if matching_count >= 20 {
        18
    } else if matching_count >= 12 {
        12
    } else {
        matching_count
    }
}

fn derive_template_defaults(matching_count: usize) -> TemplateDefaults {
    if matching_count >= 20 {
        TemplateDefaults {
            line_support: 0.8,
            word_support: 0.8,
        }
    } else if matching_count >= 10 {
        TemplateDefaults {
            line_support: 0.75,
            word_support: 0.75,
        }
    } else if matching_count >= 5 {
        TemplateDefaults {
            line_support: 0.67,
            word_support: 0.67,
        }
    } else {
        TemplateDefaults {
            line_support: 0.5,
            word_support: 0.5,
        }
    }
}

fn build_adaptive_template_from_candidates(
    sorted_emails: &[&TemplateInputEmail],
    max_template_emails: usize,
    defaults: &TemplateDefaults,
) -> (String, String) {
    let working_emails = choose_dense_financial_subgroup(sorted_emails);
    let max_n = max_template_emails.min(working_emails.len());
    let profiles = [
        (max_n, defaults.line_support, defaults.word_support, false),
        (max_n.min(12), 0.67, 0.67, false),
        (max_n.min(8), 0.60, 0.60, false),
        (max_n.min(5), 0.50, 0.50, false),
        (max_n.min(5), 0.50, 0.50, true),
        (max_n.min(3), 0.50, 0.50, true),
    ];

    let mut fallback_subject = String::new();
    let mut fallback_body = String::new();

    for (n, line_support, word_support, focused_mode) in profiles {
        if n == 0 {
            continue;
        }
        let emails: Vec<TemplateInputEmail> = working_emails
            .iter()
            .take(n)
            .map(|e| TemplateInputEmail {
                id: e.id,
                subject: e.subject.clone(),
                body: if focused_mode {
                    preprocess_body_for_template(&e.body, true)
                } else {
                    preprocess_body_for_template(&e.body, false)
                },
            })
            .collect();
        let subjects: Vec<String> = emails.iter().map(|e| e.subject.clone()).collect();
        let bodies: Vec<String> = emails.iter().map(|e| e.body.clone()).collect();

        let mut placeholder_counter = 1usize;
        let subject_template = build_subject_template_with_support(
            &subjects,
            word_support,
            emails.len(),
            &mut placeholder_counter,
        );
        let body_template = build_template_word_mode_with_support(
            &bodies,
            line_support,
            word_support,
            emails.len(),
            &mut placeholder_counter,
        );

        fallback_subject = subject_template.clone();
        fallback_body = body_template.clone();

        if !is_placeholder_only_template(&body_template) {
            return (subject_template, body_template);
        }
    }

    (fallback_subject, fallback_body)
}

fn choose_dense_financial_subgroup<'a>(
    sorted_emails: &'a [&'a TemplateInputEmail],
) -> Vec<&'a TemplateInputEmail> {
    let mut by_sig: HashMap<String, Vec<&TemplateInputEmail>> = HashMap::new();
    for e in sorted_emails {
        if let Some(sig) = financial_signature(&e.body) {
            by_sig.entry(sig).or_default().push(*e);
        }
    }
    let mut best: Option<Vec<&TemplateInputEmail>> = None;
    for group in by_sig.into_values() {
        if best.as_ref().is_none_or(|b| group.len() > b.len()) {
            best = Some(group);
        }
    }
    match best {
        Some(group) if group.len() >= 3 => group,
        _ => sorted_emails.to_vec(),
    }
}

fn financial_signature(body: &str) -> Option<String> {
    let maybe_line = body
        .lines()
        .map(|l| l.trim())
        .find(|l| {
            let s = l.to_ascii_lowercase();
            s.contains("bill dated") && (s.contains("amount") || s.contains("due"))
        })
        .or_else(|| {
            body.lines().map(|l| l.trim()).find(|l| {
                let s = l.to_ascii_lowercase();
                s.contains("due date") && s.contains("payment")
            })
        })?;
    let mut sig = maybe_line.to_ascii_lowercase();
    if let Ok(re_num) = Regex::new(r"\d+") {
        sig = re_num.replace_all(&sig, "#").to_string();
    }
    if let Ok(re_month) = Regex::new(r"\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\b")
    {
        sig = re_month.replace_all(&sig, "mon").to_string();
    }
    if let Ok(re_ws) = Regex::new(r"\s+") {
        sig = re_ws.replace_all(&sig, " ").to_string();
    }
    Some(sig.trim().to_string())
}

fn is_placeholder_only_template(template: &str) -> bool {
    let re = match Regex::new(r"^\{\{\s*placeholder_\d+\s*\}\}$") {
        Ok(re) => re,
        Err(_) => return false,
    };
    let lines: Vec<&str> = template.lines().filter(|l| !l.trim().is_empty()).collect();
    !lines.is_empty() && lines.iter().all(|line| re.is_match(line.trim()))
}

fn preprocess_body_for_template(raw: &str, focused_mode: bool) -> String {
    let mut lines: Vec<String> = raw
        .replace('\r', "")
        .lines()
        .map(normalize_line)
        .filter(|l| !l.is_empty())
        .filter(|l| !is_noise_line(l))
        .collect();

    let merged = lines.join(" ");
    lines = split_into_sentences(&merged)
        .into_iter()
        .map(|s| normalize_line(&s))
        .filter(|s| !s.is_empty())
        .filter(|s| !is_noise_line(s))
        .collect();

    if focused_mode {
        let focused: Vec<String> = lines
            .iter()
            .filter(|l| is_financial_signal_line(l))
            .cloned()
            .collect();
        if !focused.is_empty() {
            lines = focused;
        }
    }

    lines.join("\n")
}

fn split_into_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        buf.push(ch);
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let next = if i + 1 < chars.len() {
            Some(chars[i + 1])
        } else {
            None
        };
        let is_sentence_punct = matches!(ch, '.' | '!' | '?');
        let punct_followed_by_space_or_end = next.is_none_or(|c| c.is_whitespace());
        let is_domain_or_decimal = ch == '.'
            && prev.is_some_and(|c| c.is_ascii_alphanumeric())
            && next.is_some_and(|c| c.is_ascii_alphanumeric());
        let is_abbreviation = ch == '.'
            && should_keep_period_with_prev_token(&buf)
            && next_non_whitespace_char(&chars, i + 1).is_some();
        let boundary = is_sentence_punct
            && punct_followed_by_space_or_end
            && !is_domain_or_decimal
            && !is_abbreviation;
        if boundary {
            let s = buf.trim();
            if !s.is_empty() {
                out.push(s.to_string());
            }
            buf.clear();
            while i + 1 < chars.len() && chars[i + 1].is_whitespace() {
                i += 1;
            }
        }
        i += 1;
    }
    let tail = buf.trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

fn should_keep_period_with_prev_token(buf: &str) -> bool {
    let token = last_alpha_token_before_period(buf)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if token.is_empty() {
        return false;
    }
    if token.len() == 1 {
        return true;
    }
    matches!(
        token.as_str(),
        "no" | "mr"
            | "mrs"
            | "ms"
            | "dr"
            | "prof"
            | "sr"
            | "jr"
            | "st"
            | "vs"
            | "etc"
            | "fig"
            | "dept"
            | "inc"
            | "ltd"
            | "co"
    )
}

fn last_alpha_token_before_period(buf: &str) -> Option<String> {
    let without_period = buf.strip_suffix('.')?.trim_end();
    if without_period.is_empty() {
        return None;
    }
    let token = without_period
        .rsplit(|c: char| !c.is_ascii_alphabetic())
        .find(|part| !part.is_empty())?;
    Some(token.to_string())
}

fn next_non_whitespace_char(chars: &[char], start: usize) -> Option<char> {
    let mut idx = start;
    while idx < chars.len() {
        let ch = chars[idx];
        if !ch.is_whitespace() {
            return Some(ch);
        }
        idx += 1;
    }
    None
}

fn normalize_line(line: &str) -> String {
    let mut s = line
        .replace('\u{00A0}', " ")
        .replace('\u{FFFD}', " ")
        .trim()
        .to_string();
    if let Ok(re) = Regex::new(r"\s+") {
        s = re.replace_all(&s, " ").to_string();
    }
    s
}

fn is_noise_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.len() > 260 {
        return true;
    }
    if lower.chars().all(|c| !c.is_ascii_alphanumeric()) {
        return true;
    }
    lower.contains("confidentiality notice")
        || lower.contains("intended recipient")
        || lower.contains("unauthorized")
        || lower.contains("privileged information")
        || lower.contains("this is an auto generated email")
}

fn is_financial_signal_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("bill dated")
        || lower.contains("amount")
        || lower.contains("due date")
        || lower.contains("due for payment")
        || lower.contains("payment")
        || lower.contains("rs ")
        || lower.contains("rs,")
        || lower.contains("airtel mobile")
        || lower.contains("fixedline")
}

fn support_count(total_emails: usize, support_ratio: f64) -> usize {
    let required = ((total_emails as f64) * support_ratio).ceil().max(1.0) as usize;
    if total_emails >= 2 {
        required.max(2).min(total_emails)
    } else {
        required.min(total_emails.max(1))
    }
}

fn build_subject_template_with_support(
    subjects: &[String],
    word_support: f64,
    total_emails: usize,
    placeholder_counter: &mut usize,
) -> String {
    if subjects.is_empty() {
        return String::new();
    }
    build_token_support_template(
        &subjects.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        word_support,
        total_emails,
        "subject",
        placeholder_counter,
    )
}

fn build_template_word_mode_with_support(
    bodies: &[String],
    line_support: f64,
    word_support: f64,
    total_emails: usize,
    placeholder_counter: &mut usize,
) -> String {
    let required_line_support = support_count(total_emails, line_support);
    let all_lines: Vec<Vec<&str>> = bodies.iter().map(|b| b.lines().collect()).collect();
    let max_lines = all_lines.iter().map(|lines| lines.len()).max().unwrap_or(0);
    let mut template_lines = Vec::new();

    for line_idx in 0..max_lines {
        let versions: Vec<&str> = all_lines
            .iter()
            .filter_map(|lines| lines.get(line_idx).copied())
            .filter(|line| !line.trim().is_empty())
            .collect();

        if versions.len() < required_line_support {
            continue;
        }

        let line_template = build_token_support_template(
            &versions,
            word_support,
            total_emails,
            "placeholder",
            placeholder_counter,
        );
        if !line_template.trim().is_empty() {
            template_lines.push(line_template);
        }
    }

    template_lines.join("\n")
}

fn build_token_support_template(
    versions: &[&str],
    token_support: f64,
    total_emails: usize,
    placeholder_prefix: &str,
    placeholder_counter: &mut usize,
) -> String {
    let required_token_support = support_count(total_emails, token_support);
    let tokenized: Vec<Vec<&str>> = versions
        .iter()
        .map(|line| line.split_whitespace().collect())
        .collect();
    let max_tokens = tokenized.iter().map(|t| t.len()).max().unwrap_or(0);
    let mut out_tokens: Vec<String> = Vec::new();
    let mut in_placeholder_run = false;

    for token_idx in 0..max_tokens {
        let mut bucket: HashMap<&str, usize> = HashMap::new();
        for tokens in &tokenized {
            if let Some(token) = tokens.get(token_idx) {
                *bucket.entry(token).or_insert(0) += 1;
            }
        }

        let best = bucket
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(token, count)| (token.to_string(), count));

        if let Some((token, count)) = best {
            if count >= required_token_support {
                out_tokens.push(token);
                in_placeholder_run = false;
                continue;
            }
        }

        if !in_placeholder_run {
            out_tokens.push(format!(
                "{{{{ {}_{} }}}}",
                placeholder_prefix, placeholder_counter
            ));
            *placeholder_counter += 1;
            in_placeholder_run = true;
        }
    }

    out_tokens.join(" ")
}

fn comparable_text(email: &TemplateInputEmail) -> String {
    format!("{}\n{}", email.subject, email.body)
}

fn normalized_word_edit_distance(a: &str, b: &str) -> f64 {
    let a_tokens: Vec<&str> = a.split_whitespace().collect();
    let b_tokens: Vec<&str> = b.split_whitespace().collect();
    if a_tokens.is_empty() && b_tokens.is_empty() {
        return 0.0;
    }
    let dist = levenshtein_words(&a_tokens, &b_tokens) as f64;
    dist / a_tokens.len().max(b_tokens.len()) as f64
}

fn levenshtein_words(a: &[&str], b: &[&str]) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = vec![0; b.len() + 1];
    for (i, a_tok) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, b_tok) in b.iter().enumerate() {
            let cost = if a_tok == b_tok { 0 } else { 1 };
            curr[j + 1] = (curr[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

pub fn reconstruct_template_from_variables(
    subject: &str,
    body: &str,
    variables: &[TemplateVariable],
) -> String {
    let mut result = String::new();
    result.push_str("Subject: ");

    let mut subject_replaced = subject.to_string();
    let mut body_replaced = body.to_string();

    let mut sorted_vars: Vec<_> = variables.iter().collect();
    sorted_vars.sort_by(|a, b| b.value.len().cmp(&a.value.len()));

    for var in sorted_vars {
        subject_replaced =
            subject_replaced.replace(&var.value, &format!("{{{{{}}}}}", var.variable_name));
        body_replaced =
            body_replaced.replace(&var.value, &format!("{{{{{}}}}}", var.variable_name));
    }

    result.push_str(&subject_replaced);
    result.push_str("\n---\n");
    result.push_str(&body_replaced);

    result
}
