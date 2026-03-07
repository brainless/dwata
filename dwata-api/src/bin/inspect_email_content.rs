use anyhow::{Context, Result};
use clap::Parser;
use dwata_agents::storage::{AgentStorage, InMemoryAgentStorage, Session};
use dwata_agents::{
    extract_values_from_email, normalize_email_content, LlmReverseTemplateExtractorAgent,
    ReverseTemplateType, TemplateDocumentLabelerAgent, TemplateEmailContent,
};
use dwata_api::database::emails as emails_db;
use dwata_api::helpers::database::initialize_database;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use regex::Regex;
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

#[derive(Debug, Parser)]
#[command(
    name = "inspect_email_template",
    about = "Print cleaned email content plus detected type and generated canonical template"
)]
struct Args {
    /// Email ID from the emails table
    email_id: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let db = initialize_database().context("Failed to initialize database")?;

    let email = emails_db::get_email(db.async_connection.clone(), args.email_id)
        .await
        .with_context(|| format!("Failed to load email id={}", args.email_id))?;

    let original_body =
        preferred_original_body(email.body_text.as_deref(), email.body_html.as_deref());
    let normalized = normalize_email_content(
        email.subject.as_deref(),
        email.body_text.as_deref(),
        email.body_html.as_deref(),
    );

    println!("Email ID: {}", email.id);
    println!("Subject:");
    println!("{}", email.subject.as_deref().unwrap_or_default());
    println!();
    println!("Original Body Source: {}", original_body.0);
    println!("Original Body:");
    println!("{}", original_body.1);
    println!();
    println!("Normalized Subject:");
    println!("{}", normalized.subject);
    println!();
    println!("Normalized Body:");
    println!("{}", normalized.body);
    println!();

    let llm_client = Arc::new(OllamaClient::new().context("Failed to initialize Ollama client")?);
    let storage = Arc::new(InMemoryAgentStorage::new());
    let formatted_cleaned_email =
        format!("Subject: {}\n---\n{}", normalized.subject, normalized.body);

    let labeler_session_id = storage
        .create_session(Session {
            id: None,
            agent_type: "template-document-labeler".to_string(),
            objective: "Classify cleaned financial email".to_string(),
            context_data: None,
            status: "running".to_string(),
            result: None,
        })
        .await
        .context("Failed to create template-document-labeler session")?;
    let labeler = TemplateDocumentLabelerAgent::new(
        llm_client.clone(),
        storage.clone(),
        MINISTRAL_3_3B_ID.to_string(),
        formatted_cleaned_email,
    );
    let label = labeler
        .execute(labeler_session_id)
        .await
        .context("Template document labeler failed")?;

    println!("Detected Document Type:");
    println!(
        "{:?} (has_bill={}, has_transaction={})",
        label.doc_type, label.has_bill, label.has_transaction
    );

    let mut template_types = Vec::new();
    if label.has_bill {
        template_types.push(ReverseTemplateType::Bill);
    }
    if label.has_transaction {
        template_types.push(ReverseTemplateType::Transaction);
    }

    if template_types.is_empty() {
        println!();
        println!("No Bill/Transaction template generation requested by labeler result.");
        return Ok(());
    }

    for template_type in template_types {
        let reverse_session_id = storage
            .create_session(Session {
                id: None,
                agent_type: "llm-reverse-template-extractor".to_string(),
                objective: format!("Generate {:?} template from cleaned email", template_type),
                context_data: None,
                status: "running".to_string(),
                result: None,
            })
            .await
            .context("Failed to create llm-reverse-template-extractor session")?;
        let reverse = LlmReverseTemplateExtractorAgent::new(
            llm_client.clone(),
            storage.clone(),
            template_type,
            normalized.subject.clone(),
            normalized.body.clone(),
        );
        let generated = reverse.execute(reverse_session_id).await.with_context(|| {
            format!("Reverse template extraction failed for {:?}", template_type)
        })?;
        let variables = extract_template_variables(&generated.template_body);

        println!();
        println!("Generated Template Type: {:?}", template_type);
        println!("Generated Template:");
        println!("{}", generated.template_body);
        println!("Variables:");
        if variables.is_empty() {
            println!("(none)");
        } else {
            for variable in variables {
                println!("- {}", variable);
            }
        }

        println!();
        println!("Extracted Values (from original email using template):");
        let extracted = extract_values_from_email(
            &generated.template_body,
            &TemplateEmailContent {
                subject: normalized.subject.clone(),
                body: normalized.body.clone(),
            },
        );
        if extracted.is_empty() {
            println!("(none extracted)");
        } else {
            println!("+---------------------------+----------------------------------+");
            println!("| {:<25} | {:<30} |", "Placeholder", "Extracted Value");
            println!("+---------------------------+----------------------------------+");
            for (key, value) in &extracted {
                let display_value = if value.len() > 30 {
                    format!("{}...", &value[..27])
                } else {
                    value.clone()
                };
                println!("| {:<25} | {:<30} |", key, display_value);
            }
            println!("+---------------------------+----------------------------------+");

            let vendor_names = extracted_vendor_names(&extracted);
            if !vendor_names.is_empty() {
                println!();
                println!("Vendor Name(s): {}", vendor_names.join(", "));
            }
        }
    }

    Ok(())
}

fn preferred_original_body<'a>(
    body_text: Option<&'a str>,
    body_html: Option<&'a str>,
) -> (&'static str, &'a str) {
    if let Some(text) = body_text {
        if !text.trim().is_empty() {
            return ("body_text", text);
        }
    }
    if let Some(html) = body_html {
        return ("body_html", html);
    }
    ("none", "")
}

fn template_variable_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{\s*([a-zA-Z0-9_-]+)\s*\}\}").expect("valid regex"))
}

fn extract_template_variables(template: &str) -> Vec<String> {
    let mut variables = BTreeSet::new();
    for caps in template_variable_regex().captures_iter(template) {
        if let Some(name) = caps.get(1) {
            variables.insert(name.as_str().to_string());
        }
    }
    variables.into_iter().collect()
}

fn extracted_vendor_names(extracted: &std::collections::HashMap<String, String>) -> Vec<String> {
    let mut names = BTreeSet::new();
    for key in [
        "vendor_name",
        "vendor-name",
        "vendor",
        "payer_vendor_name",
        "payer-vendor-name",
        "payee_vendor_name",
        "payee-vendor-name",
    ] {
        if let Some(value) = extracted.get(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                names.insert(trimmed.to_string());
            }
        }
    }
    names.into_iter().collect()
}
