use anyhow::{Context, Result};
use chrono::NaiveDate;
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

/// Holds the subject and plain-text body extracted from an email.
struct Email {
    subject: String,
    body: String,
}

struct DbEmail {
    id: i64,
    subject: String,
    body: String,
}

struct TemplateDefaults {
    line_support: f64,
    word_support: f64,
}

struct EmailCluster {
    seed_text: String,
    members: Vec<DbEmail>,
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
    let mut clusters = cluster_emails(sender_emails, cli.word_distance_threshold)?;
    clusters.retain(|c| c.members.len() >= 2);
    if clusters.is_empty() {
        return Err(anyhow::anyhow!(
            "No reusable templates found for sender '{}' (need at least 2 emails per template).",
            cli.email_from
        ));
    }
    clusters.sort_by(|a, b| b.members.len().cmp(&a.members.len()));

    println!(
        "Using {} reusable cluster(s) (minimum 2 emails each).",
        clusters.len()
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

    for (idx, cluster) in clusters.iter().enumerate() {
        let defaults = derive_template_defaults(cluster.members.len());
        let max_template_emails = derive_max_template_emails(cluster.members.len());

        let mut scored: Vec<(f64, &DbEmail)> = cluster
            .members
            .iter()
            .map(|e| {
                let dist = normalized_word_edit_distance(&cluster.seed_text, &comparable_text(e));
                (dist, e)
            })
            .collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        let selected: Vec<&DbEmail> = scored
            .into_iter()
            .take(max_template_emails)
            .map(|(_, e)| e)
            .collect();

        let ids: Vec<String> = selected.iter().map(|e| e.id.to_string()).collect();
        println!(
            "\n=== Template {} (cluster size {}) ===",
            idx + 1,
            cluster.members.len()
        );
        println!(
            "Selected {} emails for template generation (ids: {}). line_support={:.2}, word_support={:.2}",
            selected.len(),
            ids.join(", "),
            defaults.line_support,
            defaults.word_support
        );

        let emails: Vec<Email> = selected
            .iter()
            .map(|e| Email {
                subject: e.subject.clone(),
                body: e.body.clone(),
            })
            .collect();
        let subjects: Vec<String> = emails.iter().map(|e| e.subject.clone()).collect();
        let bodies: Vec<String> = emails.iter().map(|e| e.body.clone()).collect();
        let mut placeholder_counter = 1usize;
        let subject_template = build_subject_template_with_support(
            &subjects,
            defaults.word_support,
            emails.len(),
            &mut placeholder_counter,
        );
        let body_template = build_template_word_mode_with_support(
            &bodies,
            defaults.line_support,
            defaults.word_support,
            emails.len(),
            &mut placeholder_counter,
        );
        let full_template = format!("Subject: {subject_template}\n---\n{body_template}");

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
            seed_text: cluster.seed_text.clone(),
            size: cluster.members.len(),
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
                            row.push(vals.get(field).cloned().unwrap_or_else(|| "-".to_string()));
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
                            row.push(vals.get(field).cloned().unwrap_or_else(|| "-".to_string()));
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

fn parse_amount(raw: &str) -> Option<f64> {
    let re = Regex::new(r"[^\d,\.\-]").ok()?;
    let cleaned = re.replace_all(raw, "").replace(',', "");
    if cleaned.is_empty() || cleaned == "-" || cleaned == "." || cleaned == "-." {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 60 {
        return None;
    }
    parse_datetime(trimmed).ok().map(|dt| dt.date_naive())
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
        "SELECT id, COALESCE(subject, ''), COALESCE(body_text, '')
         FROM emails
         WHERE LOWER(from_address) = LOWER(?1)
         ORDER BY date_received DESC",
    )?;
    let rows = stmt.query_map(params![sender_email], |row| {
        Ok(DbEmail {
            id: row.get(0)?,
            subject: row.get(1)?,
            body: row.get(2)?,
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
            "SELECT id, COALESCE(subject, ''), COALESCE(body_text, '')
             FROM emails
             WHERE LOWER(from_address) = LOWER(?1)
             ORDER BY date_received DESC",
        )?;
        let rows = stmt.query_map(params![sender_email], |row| {
            Ok(DbEmail {
                id: row.get(0)?,
                subject: row.get(1)?,
                body: row.get(2)?,
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
            "SELECT id, COALESCE(subject, ''), COALESCE(body_text, '')
             FROM emails
             WHERE LOWER(from_address) = LOWER(?1)
             ORDER BY date_received DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![sender_email, max_db_emails_i64], |row| {
            Ok(DbEmail {
                id: row.get(0)?,
                subject: row.get(1)?,
                body: row.get(2)?,
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

fn cluster_emails(candidates: Vec<DbEmail>, threshold: f64) -> Result<Vec<EmailCluster>> {
    if !(0.0..=1.0).contains(&threshold) {
        return Err(anyhow::anyhow!(
            "--word-distance-threshold must be between 0.0 and 1.0"
        ));
    }

    let total_count = candidates.len();
    if total_count < 2 {
        return Err(anyhow::anyhow!(
            "Need at least 2 non-empty emails, found {}.",
            total_count
        ));
    }

    // clusters: (seed_text, members)
    let mut clusters: Vec<EmailCluster> = Vec::new();

    // Early-abort at 1/3 of total: if every email so far is its own singleton cluster,
    // there is no repeating template from this sender.
    let early_abort_at = (total_count / 3).max(2);

    for (idx, email) in candidates.into_iter().enumerate() {
        let email_text = comparable_text(&email);

        // Find the closest existing cluster.
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

        // Early-abort check: at the 1/3 mark, if every cluster is still a singleton,
        // no repeating pattern exists — bail out rather than scanning all emails.
        if idx + 1 == early_abort_at {
            let all_singletons = clusters.iter().all(|c| c.members.len() == 1);
            if all_singletons {
                return Err(anyhow::anyhow!(
                    "No similar emails found after checking {}/{} — \
                     all emails are unique so far. Try a higher --word-distance-threshold.",
                    idx + 1,
                    total_count
                ));
            }
        }
    }

    println!("Formed {} cluster(s).", clusters.len());
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

fn support_count(total_emails: usize, support_ratio: f64) -> usize {
    ((total_emails as f64) * support_ratio).ceil().max(1.0) as usize
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
        let mut bucket: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
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

/// Build a Jinja2 template for the subject line by word-diffing across all
/// email subjects.
fn build_subject_template(subjects: &[String]) -> String {
    if subjects.is_empty() {
        return String::new();
    }
    // All identical → return as-is
    if subjects.iter().all(|s| s == &subjects[0]) {
        return subjects[0].clone();
    }

    let tokenized: Vec<Vec<&str>> = subjects
        .iter()
        .map(|s| s.split_whitespace().collect())
        .collect();

    let mut common = lcs(&tokenized[0], &tokenized[1]);
    for tokens in &tokenized[2..] {
        let refs: Vec<&str> = common.iter().map(|s| s.as_str()).collect();
        common = lcs(&refs, tokens);
    }

    // Align against first subject, replacing gaps with placeholders
    let mut parts: Vec<String> = Vec::new();
    let mut counter: usize = 1;
    let mut ti = 0usize;
    let mut ci = 0usize;

    while ti < tokenized[0].len() {
        if ci < common.len() && tokenized[0][ti] == common[ci].as_str() {
            parts.push(tokenized[0][ti].to_string());
            ci += 1;
            ti += 1;
        } else {
            while ti < tokenized[0].len()
                && (ci >= common.len() || tokenized[0][ti] != common[ci].as_str())
            {
                ti += 1;
            }
            parts.push(format!("{{{{ subject_{} }}}}", counter));
            counter += 1;
        }
    }

    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Word-mode template generation
// ---------------------------------------------------------------------------

/// Splits every line into words, diffs at word level, and replaces variable
/// words with placeholders. This gives finer-grained templates.
fn build_template_word_mode(bodies: &[String]) -> String {
    // Split into lines, then process line-by-line across all emails.
    let all_lines: Vec<Vec<&str>> = bodies.iter().map(|b| b.lines().collect()).collect();

    // First find the common *lines* skeleton (exact-match lines) so we know
    // the structural anchors.
    let mut common_lines = lcs(&all_lines[0], &all_lines[1]);
    for lines in &all_lines[2..] {
        let refs: Vec<&str> = common_lines.iter().map(|s| s.as_str()).collect();
        common_lines = lcs(&refs, lines);
    }

    // Build alignment: map common_lines indices → first-email line indices.
    let alignment = align_multiple(&all_lines, &common_lines);

    let mut template_lines: Vec<String> = Vec::new();
    let mut placeholder_counter: usize = 1;

    let mut prev_idx: Option<usize> = None;
    for &(common_idx, first_email_idx) in &alignment {
        // Process the gap between the previous anchor and this one.
        let gap_start = prev_idx.map(|p| p + 1).unwrap_or(0);
        if gap_start < first_email_idx {
            process_gap(
                &all_lines,
                &alignment,
                common_idx,
                gap_start,
                first_email_idx,
                &mut template_lines,
                &mut placeholder_counter,
            );
        }
        prev_idx = Some(first_email_idx);

        // The anchor line is identical across all emails – keep as-is.
        template_lines.push(common_lines[common_idx].clone());
    }

    // Trailing gap after the last anchor.
    if let Some(last) = prev_idx {
        let first_len = all_lines[0].len();
        if last + 1 < first_len {
            process_gap(
                &all_lines,
                &alignment,
                common_lines.len(), // sentinel: past-the-end
                last + 1,
                first_len,
                &mut template_lines,
                &mut placeholder_counter,
            );
        }
    } else {
        // No common lines at all – word-diff every line positionally.
        let max_lines = all_lines.iter().map(|l| l.len()).max().unwrap_or(0);
        for li in 0..max_lines {
            let versions: Vec<&str> = all_lines
                .iter()
                .filter_map(|lines| lines.get(li).copied())
                .collect();
            if versions.len() >= 2 {
                template_lines.push(word_diff_template(&versions, &mut placeholder_counter));
            } else if let Some(v) = versions.first() {
                template_lines.push(format!("{{{{ placeholder_{} }}}}", placeholder_counter));
                let _ = v; // suppress unused warning
                placeholder_counter += 1;
            }
        }
    }

    template_lines.join("\n")
}

/// Process a gap region (lines between two anchors).  Instead of replacing
/// each gap line with a whole-line placeholder, we align the gap lines
/// across emails positionally and word-diff each pair.
fn process_gap(
    all_lines: &[Vec<&str>],
    alignment: &[(usize, usize)],
    next_common_idx: usize,
    gap_start_in_first: usize,
    gap_end_in_first: usize,
    template_lines: &mut Vec<String>,
    placeholder_counter: &mut usize,
) {
    // For each email, find the corresponding gap region.  The gap for email
    // E sits between the anchor *before* this gap and the anchor *at*
    // next_common_idx.  We find those boundary positions per email.
    let gap_slices: Vec<&[&str]> = collect_gap_slices(
        all_lines,
        alignment,
        next_common_idx,
        gap_start_in_first,
        gap_end_in_first,
    );

    let max_gap_len = gap_slices.iter().map(|s| s.len()).max().unwrap_or(0);

    for li in 0..max_gap_len {
        let versions: Vec<&str> = gap_slices
            .iter()
            .filter_map(|slice| slice.get(li).copied())
            .collect();

        if versions.len() >= 2 {
            template_lines.push(word_diff_template(&versions, placeholder_counter));
        } else {
            // Line only exists in some emails – whole-line placeholder.
            template_lines.push(format!("{{{{ placeholder_{} }}}}", placeholder_counter));
            *placeholder_counter += 1;
        }
    }
}

/// For every email, extract the slice of lines in the gap region that
/// corresponds to the gap in the first email between `gap_start..gap_end`.
fn collect_gap_slices<'a>(
    all_lines: &'a [Vec<&'a str>],
    alignment: &[(usize, usize)],
    next_common_idx: usize,
    gap_start_in_first: usize,
    gap_end_in_first: usize,
) -> Vec<&'a [&'a str]> {
    // For the first email, the slice is simply [gap_start..gap_end).
    // For other emails, we need to find the positions of the surrounding
    // anchors and extract the lines between them.

    let mut result: Vec<&[&str]> = Vec::new();

    for (email_idx, email_lines) in all_lines.iter().enumerate() {
        if email_idx == 0 {
            result.push(&email_lines[gap_start_in_first..gap_end_in_first]);
            continue;
        }

        // Find position of the previous anchor in this email (the anchor
        // just before next_common_idx).
        let prev_anchor_pos = if next_common_idx > 0 {
            find_anchor_pos_in_email(
                email_lines,
                alignment,
                next_common_idx - 1,
                all_lines[0].as_slice(),
            )
        } else {
            None
        };

        // Find position of the next anchor in this email.
        let next_anchor_pos = if next_common_idx < alignment.len() {
            find_anchor_pos_in_email(
                email_lines,
                alignment,
                next_common_idx,
                all_lines[0].as_slice(),
            )
        } else {
            None
        };

        let start = prev_anchor_pos.map(|p| p + 1).unwrap_or(0);
        let end = next_anchor_pos.unwrap_or(email_lines.len());
        if start <= end && end <= email_lines.len() {
            result.push(&email_lines[start..end]);
        } else {
            result.push(&[]);
        }
    }

    result
}

/// Find the position of a given anchor (by common_idx) in a specific email.
/// The anchor's text is `all_lines[0][alignment[common_idx].1]`.  We search
/// forward from the previous anchor's position in this email.
fn find_anchor_pos_in_email(
    email_lines: &[&str],
    alignment: &[(usize, usize)],
    common_idx: usize,
    first_email_lines: &[&str],
) -> Option<usize> {
    if common_idx >= alignment.len() {
        return None;
    }
    let (_, first_pos) = alignment[common_idx];
    let anchor_text = first_email_lines[first_pos];

    // Determine search start: after the previous anchor in this email.
    let search_start = if common_idx > 0 {
        // Recursively find prev anchor position, then start after it.
        find_anchor_pos_in_email(email_lines, alignment, common_idx - 1, first_email_lines)
            .map(|p| p + 1)
            .unwrap_or(0)
    } else {
        0
    };

    for i in search_start..email_lines.len() {
        if email_lines[i] == anchor_text {
            return Some(i);
        }
    }
    None
}

/// Given several versions of the same logical line, diff at word level and
/// produce a template line.
fn word_diff_template(line_versions: &[&str], counter: &mut usize) -> String {
    let tokenized: Vec<Vec<&str>> = line_versions
        .iter()
        .map(|l| l.split_whitespace().collect())
        .collect();

    // Pairwise LCS across all versions
    let mut common_words = lcs(&tokenized[0], &tokenized[1]);
    for tokens in &tokenized[2..] {
        let refs: Vec<&str> = common_words.iter().map(|s| s.as_str()).collect();
        common_words = lcs(&refs, tokens);
    }

    // Align against the first version
    let mut result_parts: Vec<String> = Vec::new();
    let mut ti = 0usize; // index into tokenized[0]
    let mut ci = 0usize; // index into common_words

    while ti < tokenized[0].len() {
        if ci < common_words.len() && tokenized[0][ti] == common_words[ci].as_str() {
            result_parts.push(tokenized[0][ti].to_string());
            ci += 1;
            ti += 1;
        } else {
            // Consume all non-common words as a single placeholder
            let mut gap = Vec::new();
            while ti < tokenized[0].len()
                && (ci >= common_words.len() || tokenized[0][ti] != common_words[ci].as_str())
            {
                gap.push(tokenized[0][ti]);
                ti += 1;
            }
            if !gap.is_empty() {
                result_parts.push(format!("{{{{ placeholder_{} }}}}", counter));
                *counter += 1;
            }
        }
    }

    // If there are remaining common words (shouldn't happen in a correct LCS)
    // just append them.
    while ci < common_words.len() {
        result_parts.push(common_words[ci].clone());
        ci += 1;
    }

    result_parts.join(" ")
}

// ---------------------------------------------------------------------------
// Alignment helpers
// ---------------------------------------------------------------------------

/// Build an alignment of common_lines indices → first-email line indices.
fn align_multiple(all_lines: &[Vec<&str>], common_lines: &[String]) -> Vec<(usize, usize)> {
    let first = &all_lines[0];
    let mut result = Vec::new();
    let mut fi = 0usize;
    for (ci, cl) in common_lines.iter().enumerate() {
        while fi < first.len() {
            if first[fi] == cl.as_str() {
                result.push((ci, fi));
                fi += 1;
                break;
            }
            fi += 1;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// LCS (Longest Common Subsequence)
// ---------------------------------------------------------------------------

/// Classic DP-based LCS that works on slices of string-like items.
fn lcs<T: AsRef<str>>(a: &[T], b: &[T]) -> Vec<String> {
    let m = a.len();
    let n = b.len();

    // Build DP table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1].as_ref() == b[j - 1].as_ref() {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Back-track to recover the subsequence
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i - 1].as_ref() == b[j - 1].as_ref() {
            result.push(a[i - 1].as_ref().to_string());
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcs_identical() {
        let a = vec!["hello", "world"];
        let b = vec!["hello", "world"];
        assert_eq!(lcs(&a, &b), vec!["hello", "world"]);
    }

    #[test]
    fn test_lcs_partial() {
        let a = vec!["a", "b", "c", "d"];
        let b = vec!["a", "x", "c", "y"];
        assert_eq!(lcs(&a, &b), vec!["a", "c"]);
    }

    #[test]
    fn test_lcs_no_common() {
        let a = vec!["a", "b"];
        let b = vec!["c", "d"];
        let result = lcs(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn test_word_mode_template() {
        let email1 =
            "Dear Customer,\nYour payment of $100.00 was received.\nThank you.".to_string();
        let email2 =
            "Dear Customer,\nYour payment of $250.00 was received.\nThank you.".to_string();

        let template = build_template_word_mode(&[email1, email2]);
        assert!(template.contains("Dear Customer,"));
        assert!(template.contains("Thank you."));
        // The amount should be replaced with a placeholder
        assert!(template.contains("placeholder_"));
        assert!(!template.contains("$100.00"));
        assert!(!template.contains("$250.00"));
    }

    #[test]
    fn test_three_emails() {
        let e1 = "Hello,\nYour balance is $100.\nAccount: 111\nBye.".to_string();
        let e2 = "Hello,\nYour balance is $200.\nAccount: 222\nBye.".to_string();
        let e3 = "Hello,\nYour balance is $300.\nAccount: 333\nBye.".to_string();

        let template = build_template_word_mode(&[e1, e2, e3]);
        assert!(template.contains("Hello,"));
        assert!(template.contains("Bye."));
        assert!(template.contains("placeholder_"));
    }

    #[test]
    fn test_word_level_precision_in_gap_lines() {
        // "Amount: $7.3" vs "Amount: $1.78" should produce
        // "Amount: {{ placeholder_N }}" not "{{ placeholder_N }}"
        let e1 = "Hello,\nAmount: $7.3\nBye.".to_string();
        let e2 = "Hello,\nAmount: $1.78\nBye.".to_string();

        let template = build_template_word_mode(&[e1, e2]);
        assert!(
            template.contains("Amount:"),
            "template should keep 'Amount:'"
        );
        assert!(
            template.contains("Amount: {{ placeholder_"),
            "template should be 'Amount: {{{{ placeholder_N }}}}', got:\n{template}"
        );
        assert!(!template.contains("$7.3"));
        assert!(!template.contains("$1.78"));
    }

    #[test]
    fn test_subject_template_identical() {
        let subjects = vec![
            "Payment received".to_string(),
            "Payment received".to_string(),
        ];
        assert_eq!(build_subject_template(&subjects), "Payment received");
    }

    #[test]
    fn test_subject_template_variable() {
        let subjects = vec![
            "Payment of $100 received".to_string(),
            "Payment of $500 received".to_string(),
        ];
        let tpl = build_subject_template(&subjects);
        assert!(tpl.contains("Payment"));
        assert!(tpl.contains("received"));
        assert!(tpl.contains("subject_"));
        assert!(!tpl.contains("$100"));
        assert!(!tpl.contains("$500"));
    }
}
