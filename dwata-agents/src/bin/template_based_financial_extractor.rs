use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate};
use clap::Parser;
use config::{Config, File};
use dateparser::parse as parse_datetime;
use regex::Regex;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use dwata_agents::storage::{InMemoryAgentStorage, Session};
use dwata_agents::template_bill_extractor::{
    TemplateBillExtractorAgent, TranslateBillVariablesParams,
};
use dwata_agents::template_document_labeler::TemplateDocumentLabelerAgent;
use dwata_agents::template_financial_extractor::{
    TemplateFinancialExtractorAgent, TranslateVariablesParams,
};
use dwata_agents::{discover_template_drafts, TemplateDetectionOptions, TemplateInputEmail};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::GEMINI_3_FLASH_ID;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::models::openai::GPT_5_MINI_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use nocodo_llm_sdk::openai::OpenAIClient;

#[derive(Parser, Debug)]
#[command(
    name = "template-based-financial-extractor",
    about = "Generate a Jinja2 template from multiple emails assumed to share the same template, \
             then use an LLM agent to translate placeholder variables to financial field names.\n\n\
             Scans DB emails for a sender via --email-from, selects a cluster of similar \
             emails, then builds a support-based template that drops low-frequency noise."
)]
struct Cli {
    /// Sender email address to scan in DB (required)
    #[arg(long, required = true)]
    email_from: String,

    /// Max sender emails to scan from DB (most recent first). Use 0 to read all.
    #[arg(long, default_value_t = 0)]
    max_db_emails: usize,

    /// Normalized word-edit distance threshold used to include emails in sender cluster
    #[arg(long, default_value_t = 0.35)]
    word_distance_threshold: f64,

    /// Skip the LLM agent step and only output the raw template
    #[arg(long, default_value_t = false)]
    template_only: bool,

    /// LLM provider to use
    #[arg(long, default_value = "gemini", value_parser = ["gemini", "openai", "ollama"])]
    provider: String,

    /// Model ID to use (provider-specific)
    #[arg(long)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiConfig {
    ai_provider_api_keys: Option<AiProviderApiKeysConfig>,
}

#[derive(Debug, Deserialize)]
struct AiProviderApiKeysConfig {
    gemini_api_key: Option<String>,
    openai_api_key: Option<String>,
}

struct DbEmail {
    id: i64,
    subject: String,
    body: String,
}

struct TemplateRuntime {
    id: usize,
    seed_text: String,
    size: usize,
    full_template: String,
    translated_template: String,
    bill_placeholder_to_field: HashMap<String, String>,
    txn_placeholder_to_field: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();

    let cli = Cli::parse();

    let sender_emails = load_sender_emails_from_db(&cli.email_from, cli.max_db_emails)?;
    let sender_input_emails = sender_emails
        .iter()
        .map(|e| TemplateInputEmail {
            id: e.id,
            subject: e.subject.clone(),
            body: e.body.clone(),
        })
        .collect::<Vec<_>>();
    let drafts = discover_template_drafts(
        sender_input_emails,
        TemplateDetectionOptions {
            word_distance_threshold: cli.word_distance_threshold,
            max_clusters: 0,
        },
    )?;
    if drafts.is_empty() {
        return Err(anyhow::anyhow!(
            "No reusable templates found for sender '{}' (need at least 2 emails per template).",
            cli.email_from
        ));
    }

    println!(
        "Using {} reusable cluster(s) (minimum 2 emails each).",
        drafts.len()
    );

    let mut template_runtimes: Vec<TemplateRuntime> = Vec::new();

    let model = match cli.model {
        Some(ref m) => m.clone(),
        None => match cli.provider.as_str() {
            "gemini" => GEMINI_3_FLASH_ID.to_string(),
            "openai" => GPT_5_MINI_ID.to_string(),
            "ollama" => MINISTRAL_3_3B_ID.to_string(),
            _ => GEMINI_3_FLASH_ID.to_string(),
        },
    };

    let (llm_client, storage): (
        Option<Arc<dyn LlmClient>>,
        Option<Arc<dyn dwata_agents::AgentStorage>>,
    ) = if cli.template_only {
        (None, None)
    } else {
        let config = load_api_config()?;
        println!("Using provider: {}", cli.provider);
        println!("Using model: {model}");
        let client: Arc<dyn LlmClient> = match cli.provider.as_str() {
            "gemini" => {
                let api_key = config
                    .ai_provider_api_keys
                    .as_ref()
                    .and_then(|keys| keys.gemini_api_key.as_ref())
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Missing gemini_api_key in dwata config"))?;
                Arc::new(GeminiClient::new(api_key)?)
            }
            "openai" => {
                let api_key = config
                    .ai_provider_api_keys
                    .as_ref()
                    .and_then(|keys| keys.openai_api_key.as_ref())
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Missing openai_api_key in dwata config"))?;
                Arc::new(OpenAIClient::new(api_key)?)
            }
            "ollama" => Arc::new(OllamaClient::new()?),
            _ => return Err(anyhow::anyhow!("Unsupported provider: {}", cli.provider)),
        };
        (Some(client), Some(Arc::new(InMemoryAgentStorage::new())))
    };

    for (idx, draft) in drafts.iter().enumerate() {
        let ids: Vec<String> = draft
            .selected_email_ids
            .iter()
            .map(|e| e.to_string())
            .collect();
        println!(
            "\n=== Template {} (cluster size {}) ===",
            idx + 1,
            draft.cluster_size
        );
        println!(
            "Selected {} emails for template generation (ids: {}). line_support={:.2}, word_support={:.2}",
            draft.selected_email_ids.len(),
            ids.join(", "),
            draft.line_support,
            draft.word_support
        );

        let full_template = draft.full_template.clone();

        println!("--- Generated Template ---");
        println!("{full_template}");

        let mut bill_placeholder_to_field: HashMap<String, String> = HashMap::new();
        let mut txn_placeholder_to_field: HashMap<String, String> = HashMap::new();
        let mut translated_template = full_template.clone();

        if let (Some(client), Some(storage_ref)) = (llm_client.as_ref(), storage.as_ref()) {
            println!("--- Label + field mapping ---");
            let label_session_id = storage_ref
                .create_session(Session {
                    id: None,
                    agent_type: "template-document-labeler".to_string(),
                    objective: "Classify financial document type".to_string(),
                    context_data: None,
                    status: "running".to_string(),
                    result: None,
                })
                .await?;

            let labeler = TemplateDocumentLabelerAgent::new(
                client.clone(),
                storage_ref.clone(),
                model.clone(),
                full_template.clone(),
            );

            let label = match labeler.execute(label_session_id).await {
                Ok(params) => {
                    let _ = storage_ref
                        .update_session(Session {
                            id: Some(label_session_id),
                            agent_type: "template-document-labeler".to_string(),
                            objective: String::new(),
                            context_data: None,
                            status: "completed".to_string(),
                            result: Some(serde_json::to_string(&params)?),
                        })
                        .await;
                    Some(params)
                }
                Err(err) => {
                    let _ = storage_ref
                        .update_session(Session {
                            id: Some(label_session_id),
                            agent_type: "template-document-labeler".to_string(),
                            objective: String::new(),
                            context_data: None,
                            status: "failed".to_string(),
                            result: Some(err.to_string()),
                        })
                        .await;
                    eprintln!(
                        "Labeler failed for template {}. Continuing without mapped fields: {err}",
                        idx + 1
                    );
                    None
                }
            };

            if let Some(label) = label {
                println!("  doc_type:        {:?}", label.doc_type);
                println!("  has_bill:        {}", label.has_bill);
                println!("  has_transaction: {}", label.has_transaction);

                let mut bill_params_opt: Option<TranslateBillVariablesParams> = None;
                let mut txn_params_opt: Option<TranslateVariablesParams> = None;

                if label.has_bill {
                    let bill_session_id = storage_ref
                        .create_session(Session {
                            id: None,
                            agent_type: "template-bill-extractor".to_string(),
                            objective: "Map template placeholders to bill fields".to_string(),
                            context_data: None,
                            status: "running".to_string(),
                            result: None,
                        })
                        .await?;

                    let bill_agent = TemplateBillExtractorAgent::new(
                        client.clone(),
                        storage_ref.clone(),
                        model.clone(),
                        full_template.clone(),
                    );

                    match bill_agent.execute(bill_session_id).await {
                        Ok(params) => {
                            let _ = storage_ref
                                .update_session(Session {
                                    id: Some(bill_session_id),
                                    agent_type: "template-bill-extractor".to_string(),
                                    objective: String::new(),
                                    context_data: None,
                                    status: "completed".to_string(),
                                    result: Some(serde_json::to_string(&params)?),
                                })
                                .await;
                            bill_params_opt = Some(params);
                        }
                        Err(err) => {
                            let _ = storage_ref
                                .update_session(Session {
                                    id: Some(bill_session_id),
                                    agent_type: "template-bill-extractor".to_string(),
                                    objective: String::new(),
                                    context_data: None,
                                    status: "failed".to_string(),
                                    result: Some(err.to_string()),
                                })
                                .await;
                            eprintln!("Bill extractor failed for template {}: {err}", idx + 1);
                        }
                    }
                }

                if label.has_transaction {
                    let txn_session_id = storage_ref
                        .create_session(Session {
                            id: None,
                            agent_type: "template-financial-extractor".to_string(),
                            objective: "Map template placeholders to transaction fields"
                                .to_string(),
                            context_data: None,
                            status: "running".to_string(),
                            result: None,
                        })
                        .await?;

                    let txn_agent = TemplateFinancialExtractorAgent::new(
                        client.clone(),
                        storage_ref.clone(),
                        model.clone(),
                        full_template.clone(),
                    );

                    match txn_agent.execute(txn_session_id).await {
                        Ok(params) => {
                            let _ = storage_ref
                                .update_session(Session {
                                    id: Some(txn_session_id),
                                    agent_type: "template-financial-extractor".to_string(),
                                    objective: String::new(),
                                    context_data: None,
                                    status: "completed".to_string(),
                                    result: Some(serde_json::to_string(&params)?),
                                })
                                .await;
                            txn_params_opt = Some(params);
                        }
                        Err(err) => {
                            let _ = storage_ref
                                .update_session(Session {
                                    id: Some(txn_session_id),
                                    agent_type: "template-financial-extractor".to_string(),
                                    objective: String::new(),
                                    context_data: None,
                                    status: "failed".to_string(),
                                    result: Some(err.to_string()),
                                })
                                .await;
                            eprintln!(
                                "Transaction extractor failed for template {}: {err}",
                                idx + 1
                            );
                        }
                    }
                }

                let mut bill_field_map: HashMap<String, String> = HashMap::new();
                let mut txn_field_map: HashMap<String, String> = HashMap::new();
                if let Some(ref params) = bill_params_opt {
                    for t in &params.translations {
                        if let Some(ref f) = t.field {
                            let s = serde_json::to_string(f)
                                .unwrap_or_default()
                                .trim_matches('"')
                                .to_string();
                            bill_field_map.insert(t.placeholder.clone(), s);
                        }
                    }
                }
                if let Some(ref params) = txn_params_opt {
                    for t in &params.translations {
                        if let Some(ref f) = t.field {
                            let s = serde_json::to_string(f)
                                .unwrap_or_default()
                                .trim_matches('"')
                                .to_string();
                            txn_field_map.insert(t.placeholder.clone(), s);
                        }
                    }
                }
                bill_placeholder_to_field.extend(bill_field_map);
                txn_placeholder_to_field.extend(txn_field_map);
                let mut all_placeholder_to_field = bill_placeholder_to_field.clone();
                all_placeholder_to_field.extend(txn_placeholder_to_field.clone());
                if !all_placeholder_to_field.is_empty() {
                    translated_template =
                        translate_template(&full_template, &all_placeholder_to_field);
                    println!("--- Translated Template ---");
                    println!("{translated_template}");
                }
            }
        }

        template_runtimes.push(TemplateRuntime {
            id: idx + 1,
            seed_text: draft.seed_text.clone(),
            size: draft.cluster_size,
            full_template,
            translated_template,
            bill_placeholder_to_field,
            txn_placeholder_to_field,
        });
    }

    if cli.template_only {
        return Ok(());
    }

    println!("\n--- Extracting structured data from all sender emails ---");
    let sample_emails = load_all_emails_from_db(&cli.email_from)?;
    println!("Loaded {} emails.\n", sample_emails.len());

    let mut bill_rows_by_template: BTreeMap<usize, Vec<(usize, i64, HashMap<String, String>)>> =
        BTreeMap::new();
    let mut txn_rows_by_template: BTreeMap<usize, Vec<(usize, i64, HashMap<String, String>)>> =
        BTreeMap::new();
    let mut rejected_bill_rows = 0usize;
    let mut rejected_txn_rows = 0usize;
    let mut unmatched = 0usize;

    for (i, email) in sample_emails.iter().enumerate() {
        let email_text = comparable_text(email);
        let best = template_runtimes
            .iter()
            .map(|t| (t, normalized_word_edit_distance(&t.seed_text, &email_text)))
            .min_by(|a, b| a.1.total_cmp(&b.1));

        if let Some((tmpl, dist)) = best {
            if dist <= cli.word_distance_threshold {
                let placeholder_vals = extract_values_from_email(&tmpl.full_template, email);
                let mut bill_field_vals: HashMap<String, String> = HashMap::new();
                let mut txn_field_vals: HashMap<String, String> = HashMap::new();
                let mut bill_row_valid = true;
                let mut txn_row_valid = true;
                for (placeholder, value) in placeholder_vals {
                    if let Some(field) = tmpl.bill_placeholder_to_field.get(&placeholder) {
                        if is_valid_bill_value(field, &value) {
                            bill_field_vals.insert(field.clone(), value.clone());
                        } else {
                            bill_row_valid = false;
                            eprintln!(
                                "Rejected bill row for email {}: value {:?} does not match field {:?}",
                                email.id, value, field
                            );
                        }
                    }
                    if let Some(field) = tmpl.txn_placeholder_to_field.get(&placeholder) {
                        if is_valid_txn_value(field, &value) {
                            txn_field_vals.insert(field.clone(), value);
                        } else {
                            txn_row_valid = false;
                            eprintln!(
                                "Rejected transaction row for email {}: value {:?} does not match field {:?}",
                                email.id, value, field
                            );
                        }
                    }
                }
                if !tmpl.bill_placeholder_to_field.is_empty() {
                    if bill_row_valid {
                        bill_rows_by_template.entry(tmpl.id).or_default().push((
                            i + 1,
                            email.id,
                            bill_field_vals,
                        ));
                    } else {
                        rejected_bill_rows += 1;
                    }
                }
                if !tmpl.txn_placeholder_to_field.is_empty() {
                    if txn_row_valid {
                        txn_rows_by_template.entry(tmpl.id).or_default().push((
                            i + 1,
                            email.id,
                            txn_field_vals,
                        ));
                    } else {
                        rejected_txn_rows += 1;
                    }
                }
            } else {
                unmatched += 1;
            }
        } else {
            unmatched += 1;
        }
    }

    for runtime in &template_runtimes {
        let mut bill_fields: BTreeSet<String> = BTreeSet::new();
        for f in runtime.bill_placeholder_to_field.values() {
            bill_fields.insert(f.clone());
        }
        if bill_fields.is_empty() {
            continue;
        }
        let ordered_fields: Vec<String> = bill_fields.into_iter().collect();
        let mut headers: Vec<String> = vec!["#".to_string(), "email_id".to_string()];
        headers.extend(ordered_fields.clone());
        let rows: Vec<Vec<String>> = bill_rows_by_template
            .get(&runtime.id)
            .map(|entries| {
                entries
                    .iter()
                    .map(|(idx, email_id, vals)| {
                        let mut row = vec![idx.to_string(), email_id.to_string()];
                        for field in &ordered_fields {
                            let value = vals.get(field).cloned().unwrap_or_else(|| "-".to_string());
                            row.push(format_table_value(field, &value));
                        }
                        row
                    })
                    .collect()
            })
            .unwrap_or_default();
        let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
        print_table(
            &format!(
                "Bill Extraction Data (T{} size {})",
                runtime.id, runtime.size
            ),
            &header_refs,
            &rows,
        );
    }

    for runtime in &template_runtimes {
        let mut txn_fields: BTreeSet<String> = BTreeSet::new();
        for f in runtime.txn_placeholder_to_field.values() {
            txn_fields.insert(f.clone());
        }
        if txn_fields.is_empty() {
            continue;
        }
        let ordered_fields: Vec<String> = txn_fields.into_iter().collect();
        let mut headers: Vec<String> = vec!["#".to_string(), "email_id".to_string()];
        headers.extend(ordered_fields.clone());
        let rows: Vec<Vec<String>> = txn_rows_by_template
            .get(&runtime.id)
            .map(|entries| {
                entries
                    .iter()
                    .map(|(idx, email_id, vals)| {
                        let mut row = vec![idx.to_string(), email_id.to_string()];
                        for field in &ordered_fields {
                            let value = vals.get(field).cloned().unwrap_or_else(|| "-".to_string());
                            row.push(format_table_value(field, &value));
                        }
                        row
                    })
                    .collect()
            })
            .unwrap_or_default();
        let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
        print_table(
            &format!(
                "Transaction Extraction Data (T{} size {})",
                runtime.id, runtime.size
            ),
            &header_refs,
            &rows,
        );
    }
    println!("Unmatched emails in sample: {}", unmatched);
    println!(
        "Rejected rows due to type mismatch: bills={}, transactions={}",
        rejected_bill_rows, rejected_txn_rows
    );

    println!("\n--- In-memory templates ---");
    for runtime in &template_runtimes {
        println!(
            "Template T{}: cluster_size={}, mapped_fields={}, body_lines={}",
            runtime.id,
            runtime.size,
            runtime.bill_placeholder_to_field.len() + runtime.txn_placeholder_to_field.len(),
            runtime.translated_template.lines().count()
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Template translation helpers
// ---------------------------------------------------------------------------

/// Replace `{{ placeholder_name }}` with `{{ field_name }}` for all mapped placeholders.
fn translate_template(template: &str, placeholder_to_field: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (placeholder, field) in placeholder_to_field {
        let old = format!("{{{{ {} }}}}", placeholder);
        let new = format!("{{{{ {} }}}}", field);
        result = result.replace(&old, &new);
    }
    result
}

/// Given a single template line and an email line, extract placeholder values
/// by using the fixed text segments as delimiters.
fn extract_values_from_line(template_line: &str, email_line: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();

    // Parse template line into interleaved fixed / placeholder parts.
    // fixed_parts[i] precedes placeholders[i]; fixed_parts.last() is the trailing fixed text.
    let mut fixed_parts: Vec<&str> = Vec::new();
    let mut placeholders: Vec<&str> = Vec::new();
    let mut remaining = template_line;

    loop {
        if let Some(start) = remaining.find("{{") {
            fixed_parts.push(&remaining[..start]);
            remaining = &remaining[start + 2..];
            if let Some(end) = remaining.find("}}") {
                placeholders.push(remaining[..end].trim());
                remaining = &remaining[end + 2..];
            } else {
                break;
            }
        } else {
            fixed_parts.push(remaining);
            break;
        }
    }

    if placeholders.is_empty() {
        return values;
    }

    let mut pos = 0usize;
    for (i, ph) in placeholders.iter().enumerate() {
        // Advance past the leading fixed text for this placeholder.
        let leading = if i < fixed_parts.len() {
            fixed_parts[i]
        } else {
            ""
        };
        if !leading.is_empty() {
            if let Some(p) = email_line[pos..].find(leading) {
                pos += p + leading.len();
            } else {
                return HashMap::new(); // fixed anchor not found — line doesn't match
            }
        }

        // Determine value end: start of the next fixed part (or end of line).
        let trailing = if i + 1 < fixed_parts.len() {
            fixed_parts[i + 1]
        } else {
            ""
        };
        let value_end = if !trailing.is_empty() {
            if let Some(p) = email_line[pos..].find(trailing) {
                pos + p
            } else {
                email_line.len()
            }
        } else {
            email_line.len()
        };

        let value = email_line[pos..value_end].trim();
        if !value.is_empty() {
            values.insert(ph.to_string(), value.to_string());
        }
        pos = value_end;
    }

    values
}

/// Extract placeholder values from a single email by matching template lines.
fn extract_values_from_email(template: &str, email: &DbEmail) -> HashMap<String, String> {
    let email_text = format!("Subject: {}\n---\n{}", email.subject, email.body);
    let mut all_values = HashMap::new();

    for template_line in template.lines() {
        if !template_line.contains("{{") {
            continue;
        }
        for email_line in email_text.lines() {
            let extracted = extract_values_from_line(template_line, email_line);
            if !extracted.is_empty() {
                all_values.extend(extracted);
                break; // matched — move on to the next template line
            }
        }
    }

    all_values
}

fn is_valid_bill_value(field: &str, value: &str) -> bool {
    match field {
        "total-amount" => parse_amount(value).is_some(),
        "currency" => is_currency_like(value),
        "issued-date" | "due-date" | "billing-period-start" | "billing-period-end" => {
            parse_date(value).is_some()
        }
        "document-reference" => is_reference_like(value),
        "service-identifier" => is_identifier_like(value),
        _ => false,
    }
}

fn is_valid_txn_value(field: &str, value: &str) -> bool {
    match field {
        "amount" => parse_amount(value).is_some(),
        "currency" => is_currency_like(value),
        "transaction-date" => parse_date(value).is_some(),
        "vendor" => !value.trim().is_empty(),
        "transaction-reference" => is_reference_like(value),
        _ => false,
    }
}

fn format_table_value(field: &str, value: &str) -> String {
    if value == "-" {
        return value.to_string();
    }
    if is_date_field(field) {
        if let Some(date) = parse_date(value) {
            return date.format("%d-%b-%Y").to_string().to_ascii_uppercase();
        }
    }
    value.to_string()
}

fn is_date_field(field: &str) -> bool {
    matches!(
        field,
        "issued-date"
            | "due-date"
            | "billing-period-start"
            | "billing-period-end"
            | "transaction-date"
    )
}

fn parse_amount(raw: &str) -> Option<f64> {
    let re = Regex::new(r"[^\d,\.\-]").ok()?;
    let cleaned = re.replace_all(raw, "").replace(',', "");
    if cleaned.is_empty() || cleaned == "-" || cleaned == "." || cleaned == "-." {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    let mut normalized = raw.trim().trim_end_matches(['.', ',', ';', ':']).trim();
    if normalized.is_empty() || normalized.len() > 60 {
        return None;
    }
    if let Some(parsed) = parse_datetime(normalized).ok().map(|dt| dt.date_naive()) {
        return Some(parsed);
    }

    let upper = normalized.to_ascii_uppercase();
    let upper = upper.as_str();
    let explicit_formats = [
        "%d-%b-%Y", "%d-%B-%Y", "%d/%b/%Y", "%d/%B/%Y", "%d %b %Y", "%d %B %Y",
    ];
    for fmt in explicit_formats {
        if let Ok(date) = NaiveDate::parse_from_str(upper, fmt) {
            if (1900..=2100).contains(&date.year()) {
                return Some(date);
            }
        }
    }

    // Retry with no internal spaces to handle values like "09- NOV -2021".
    let collapsed = upper.replace(' ', "");
    for fmt in ["%d-%b-%Y", "%d-%B-%Y", "%d/%b/%Y", "%d/%B/%Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(&collapsed, fmt) {
            if (1900..=2100).contains(&date.year()) {
                return Some(date);
            }
        }
    }

    normalized = upper;
    parse_datetime(normalized).ok().map(|dt| dt.date_naive())
}

fn is_currency_like(raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() || s.len() > 8 {
        return false;
    }
    let upper = s.to_ascii_uppercase();
    let is_iso_code = upper.len() == 3 && upper.chars().all(|c| c.is_ascii_alphabetic());
    let is_symbol = matches!(s, "$" | "€" | "£" | "¥" | "₹" | "₩" | "₽" | "₺" | "₫");
    is_iso_code || is_symbol
}

fn is_reference_like(raw: &str) -> bool {
    let s = raw.trim();
    if s.len() < 3 || s.len() > 80 {
        return false;
    }
    s.chars().any(|c| c.is_ascii_alphanumeric())
}

fn is_identifier_like(raw: &str) -> bool {
    let s = raw.trim();
    if s.len() < 3 || s.len() > 120 {
        return false;
    }
    s.chars().any(|c| c.is_ascii_alphanumeric())
}

/// Load the most-recent N emails for a sender from the database (no clustering).
fn load_all_emails_from_db(sender_email: &str) -> Result<Vec<DbEmail>> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database at {:?}", db_path))?;
    let mut stmt = conn.prepare(
        "SELECT id, COALESCE(subject, ''), COALESCE(body_text, ''), COALESCE(body_html, '')
         FROM emails
         WHERE LOWER(from_address) = LOWER(?1)
         ORDER BY date_received DESC",
    )?;
    let rows = stmt.query_map(params![sender_email], |row| {
        let body_text: String = row.get(2)?;
        let body_html: String = row.get(3)?;
        Ok(DbEmail {
            id: row.get(0)?,
            subject: row.get(1)?,
            body: preferred_body_text(&body_text, &body_html),
        })
    })?;
    let mut emails = Vec::new();
    for row in rows {
        emails.push(row?);
    }
    Ok(emails)
}

// ---------------------------------------------------------------------------
// ASCII table rendering
// ---------------------------------------------------------------------------

fn print_table_separator(widths: &[usize]) {
    print!("+");
    for w in widths {
        print!("-{}-+", "-".repeat(*w));
    }
    println!();
}

fn print_table_row(cells: &[&str], widths: &[usize]) {
    print!("|");
    for (i, cell) in cells.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or(cell.len());
        let display: String = if cell.chars().count() > w {
            cell.chars().take(w).collect()
        } else {
            cell.to_string()
        };
        print!(" {:width$} |", display, width = w);
    }
    println!();
}

fn print_table(title: &str, headers: &[&str], rows: &[Vec<String>]) {
    println!("=== {} ===", title);
    if rows.is_empty() {
        println!("  (no data extracted)");
        return;
    }
    const MAX_CELL: usize = 35;
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len().min(MAX_CELL)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.chars().count().min(MAX_CELL));
            }
        }
    }
    print_table_separator(&widths);
    let header_refs: Vec<&str> = headers.iter().copied().collect();
    print_table_row(&header_refs, &widths);
    print_table_separator(&widths);
    for row in rows {
        let cells: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
        print_table_row(&cells, &widths);
    }
    print_table_separator(&widths);
    println!();
}

fn load_api_config() -> Result<ApiConfig> {
    let config_path = if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("dwata").join("api.toml")
    } else {
        PathBuf::from("api.toml")
    };

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Config file not found at {:?}. Run dwata-api once or create it.",
            config_path
        ));
    }

    let builder = Config::builder()
        .add_source(File::from(config_path))
        .build()?;
    let config: ApiConfig = builder.try_deserialize()?;
    Ok(config)
}

fn load_sender_emails_from_db(sender_email: &str, max_db_emails: usize) -> Result<Vec<DbEmail>> {
    let db_path = get_db_path()?;
    if !db_path.exists() {
        return Err(anyhow::anyhow!(
            "Database not found at {:?}. Run dwata-api and sync emails first.",
            db_path
        ));
    }

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open SQLite database at {:?}", db_path))?;

    let mut candidates = Vec::new();
    if max_db_emails == 0 {
        let mut stmt = conn.prepare(
            "SELECT id, COALESCE(subject, ''), COALESCE(body_text, ''), COALESCE(body_html, '')
             FROM emails
             WHERE LOWER(from_address) = LOWER(?1)
             ORDER BY date_received DESC",
        )?;
        let rows = stmt.query_map(params![sender_email], |row| {
            let body_text: String = row.get(2)?;
            let body_html: String = row.get(3)?;
            Ok(DbEmail {
                id: row.get(0)?,
                subject: row.get(1)?,
                body: preferred_body_text(&body_text, &body_html),
            })
        })?;
        for row in rows {
            let candidate = row?;
            if !candidate.subject.trim().is_empty() || !candidate.body.trim().is_empty() {
                candidates.push(candidate);
            }
        }
    } else {
        let max_db_emails_i64: i64 = max_db_emails
            .try_into()
            .context("--max-db-emails is too large")?;
        let mut stmt = conn.prepare(
            "SELECT id, COALESCE(subject, ''), COALESCE(body_text, ''), COALESCE(body_html, '')
             FROM emails
             WHERE LOWER(from_address) = LOWER(?1)
             ORDER BY date_received DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![sender_email, max_db_emails_i64], |row| {
            let body_text: String = row.get(2)?;
            let body_html: String = row.get(3)?;
            Ok(DbEmail {
                id: row.get(0)?,
                subject: row.get(1)?,
                body: preferred_body_text(&body_text, &body_html),
            })
        })?;
        for row in rows {
            let candidate = row?;
            if !candidate.subject.trim().is_empty() || !candidate.body.trim().is_empty() {
                candidates.push(candidate);
            }
        }
    }

    let total_count = candidates.len();
    println!(
        "Found {} emails from sender '{}'.",
        total_count, sender_email
    );

    if total_count < 2 {
        return Err(anyhow::anyhow!(
            "Need at least 2 non-empty emails for sender '{}', found {}.",
            sender_email,
            total_count
        ));
    }

    Ok(candidates)
}

fn preferred_body_text(body_text: &str, body_html: &str) -> String {
    let text = body_text.trim();
    if !text.is_empty() {
        return body_text.to_string();
    }
    let html = body_html.trim();
    if html.is_empty() {
        return String::new();
    }
    html_to_text(html)
}

fn html_to_text(html: &str) -> String {
    let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").ok();
    let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").ok();
    let br_re = Regex::new(r"(?i)<br\s*/?>").ok();
    let block_close_re = Regex::new(r"(?i)</(p|div|li|tr|h[1-6]|table)>").ok();
    let tag_re = Regex::new(r"(?is)<[^>]+>").ok();
    let multi_nl_re = Regex::new(r"\n{3,}").ok();
    let multi_space_re = Regex::new(r"[ \t]{2,}").ok();

    let mut s = html.to_string();
    if let Some(re) = script_re {
        s = re.replace_all(&s, " ").to_string();
    }
    if let Some(re) = style_re {
        s = re.replace_all(&s, " ").to_string();
    }
    if let Some(re) = br_re {
        s = re.replace_all(&s, "\n").to_string();
    }
    if let Some(re) = block_close_re {
        s = re.replace_all(&s, "\n").to_string();
    }
    if let Some(re) = tag_re {
        s = re.replace_all(&s, " ").to_string();
    }

    s = s
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");
    s = s.replace('\r', "");
    if let Some(re) = multi_space_re {
        s = re.replace_all(&s, " ").to_string();
    }
    if let Some(re) = multi_nl_re {
        s = re.replace_all(&s, "\n\n").to_string();
    }
    s.trim().to_string()
}

fn get_db_path() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?;
    Ok(data_dir.join("dwata").join("db.sqlite"))
}

fn comparable_text(email: &DbEmail) -> String {
    format!("{}\n{}", email.subject, email.body)
}

fn normalized_word_edit_distance(a: &str, b: &str) -> f64 {
    let a_tokens: Vec<&str> = a.split_whitespace().collect();
    let b_tokens: Vec<&str> = b.split_whitespace().collect();

    if a_tokens.is_empty() && b_tokens.is_empty() {
        return 0.0;
    }

    let dist = levenshtein_words(&a_tokens, &b_tokens) as f64;
    let scale = a_tokens.len().max(b_tokens.len()) as f64;
    dist / scale
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
