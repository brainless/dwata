use anyhow::{Context, Result};
use clap::Parser;
use dwata_agents::storage::{AgentStorage, InMemoryAgentStorage, Session};
use dwata_agents::{
    extract_values_from_email_with_values, simple_email_content, LlmTemplateVariableExtractorAgent,
    ReverseTemplateType, TemplateDocumentLabelerAgent, TemplateEmailContent, TemplateVariable,
    TemplateVariableType,
};
use dwata_api::database::emails as emails_db;
use dwata_api::helpers::database::initialize_database;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use std::collections::BTreeSet;
use std::sync::Arc;

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
    let simple = simple_email_content(
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
    println!("Subject (simple):");
    println!("{}", simple.subject);
    println!();
    println!("Body (simple):");
    println!("{}", simple.body);
    println!();

    let llm_client = Arc::new(OllamaClient::new().context("Failed to initialize Ollama client")?);
    let storage = Arc::new(InMemoryAgentStorage::new());
    let formatted_cleaned_email = format!("Subject: {}\n---\n{}", simple.subject, simple.body);

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
        let var_type = match template_type {
            ReverseTemplateType::Bill => TemplateVariableType::Bill,
            ReverseTemplateType::Transaction => TemplateVariableType::Transaction,
        };
        let reverse_session_id = storage
            .create_session(Session {
                id: None,
                agent_type: "llm-template-variable-extractor".to_string(),
                objective: format!(
                    "Extract {:?} template variables from cleaned email",
                    template_type
                ),
                context_data: None,
                status: "running".to_string(),
                result: None,
            })
            .await
            .context("Failed to create llm-template-variable-extractor session")?;
        let var_extractor = LlmTemplateVariableExtractorAgent::new(
            llm_client.clone(),
            storage.clone(),
            var_type,
            simple.subject.clone(),
            simple.body.clone(),
        );
        let extracted_vars = var_extractor
            .execute(reverse_session_id)
            .await
            .with_context(|| {
                format!(
                    "Template variable extraction failed for {:?}",
                    template_type
                )
            })?;

        println!();
        println!("Extracted Variables from LLM:");
        println!("+---------------------------+----------------------------------+");
        println!("| {:<25} | {:<30} |", "Variable Name", "Extracted Value");
        println!("+---------------------------+----------------------------------+");
        for var in &extracted_vars.variables {
            let display_value = if var.value.len() > 30 {
                format!("{}...", &var.value[..27])
            } else {
                var.value.clone()
            };
            println!("| {:<25} | {:<30} |", var.variable_name, display_value);
        }
        println!("+---------------------------+----------------------------------+");

        let reconstructed_template =
            reconstruct_template(&simple.subject, &simple.body, &extracted_vars.variables);

        println!();
        println!("Reconstructed Template (via value search):");
        println!("{}", reconstructed_template);

        println!();
        println!("Extracted Values (from reconstructed template):");
        let extracted = extract_values_from_email_with_values(
            &extracted_vars.variables,
            &TemplateEmailContent {
                subject: simple.subject.clone(),
                body: simple.body.clone(),
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

fn reconstruct_template(subject: &str, body: &str, variables: &[TemplateVariable]) -> String {
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
