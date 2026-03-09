use anyhow::{Context, Result};
use clap::Parser;
use dwata_agents::storage::{AgentStorage, InMemoryAgentStorage, Session};
use dwata_agents::{
    extract_values_from_email_with_values, parse_amount, parse_date, simple_email_content,
    LlmTemplateVariableExtractorAgent, ReverseTemplateType, TemplateDocumentLabelerAgent,
    TemplateEmailContent, TemplateVariableType,
};
use dwata_api::database::emails as emails_db;
use dwata_api::helpers::database::initialize_database;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
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

        let extracted = extract_values_from_email_with_values(
            &extracted_vars.variables,
            &TemplateEmailContent {
                subject: simple.subject.clone(),
                body: simple.body.clone(),
            },
        );

        println!();
        println!("DB Preview (as would be stored):");
        if extracted.is_empty() {
            println!("(no values extracted)");
        } else {
            print_db_preview(&extracted, template_type, email.date_received);
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

fn print_db_preview(
    fields: &std::collections::HashMap<String, String>,
    template_type: ReverseTemplateType,
    date_received_ms: i64,
) {
    let get = |key: &str| -> Option<&str> { fields.get(key).map(|s| s.as_str()) };

    let format_date = |raw: Option<&str>| -> String {
        match raw {
            None => "(none)".to_string(),
            Some(r) => match parse_date(r) {
                Some(d) => format!("{} (raw: {})", d.format("%Y-%m-%d"), r),
                None => format!("(unparseable: {})", r),
            },
        }
    };

    println!("+---------------------------+------------------------------------------+");
    println!("| {:<25} | {:<40} |", "DB Column", "Value");
    println!("+---------------------------+------------------------------------------+");

    match template_type {
        ReverseTemplateType::Transaction => {
            let amount_str = get("amount")
                .and_then(|v| parse_amount(v))
                .map(|v| format!("{}", v.abs()))
                .unwrap_or_else(|| "(missing/invalid)".to_string());
            let currency = get("currency").unwrap_or("USD (default)");
            let date_raw = get("transaction_date");
            let vendor = get("vendor_name").unwrap_or("(none)");
            let txn_ref = get("transaction_reference").unwrap_or("(none)");

            println!("| {:<25} | {:<40} |", "amount", amount_str);
            println!("| {:<25} | {:<40} |", "currency", currency);
            println!(
                "| {:<25} | {:<40} |",
                "transaction_date_raw",
                date_raw.unwrap_or("(none)")
            );
            println!(
                "| {:<25} | {:<40} |",
                "transaction_date",
                format_date(date_raw)
            );
            println!("| {:<25} | {:<40} |", "vendor_name (lookup)", vendor);
            println!("| {:<25} | {:<40} |", "transaction_reference", txn_ref);
        }
        ReverseTemplateType::Bill => {
            let amount_str = get("total_amount")
                .and_then(|v| parse_amount(v))
                .map(|v| format!("{}", v.abs()))
                .unwrap_or_else(|| "(missing/invalid)".to_string());
            let currency = get("currency").unwrap_or("USD (default)");
            let issued_raw = get("issued_date");
            let due_raw = get("due_date");
            let doc_ref = get("document_reference").unwrap_or("(none)");

            // Fallback due date from email received timestamp
            let due_fallback = chrono::DateTime::from_timestamp_millis(date_received_ms)
                .map(|dt| dt.date_naive().format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "(none)".to_string());
            let due_display = match due_raw {
                Some(r) => format_date(Some(r)),
                None => format!("{} (fallback: email received date)", due_fallback),
            };

            println!("| {:<25} | {:<40} |", "total_amount", amount_str);
            println!("| {:<25} | {:<40} |", "currency", currency);
            println!(
                "| {:<25} | {:<40} |",
                "issued_date_raw",
                issued_raw.unwrap_or("(none)")
            );
            println!(
                "| {:<25} | {:<40} |",
                "issued_date",
                format_date(issued_raw)
            );
            println!(
                "| {:<25} | {:<40} |",
                "due_date_raw",
                due_raw.unwrap_or("(none)")
            );
            println!("| {:<25} | {:<40} |", "due_date", due_display);
            println!("| {:<25} | {:<40} |", "document_reference", doc_ref);
        }
    }

    println!("+---------------------------+------------------------------------------+");
}
