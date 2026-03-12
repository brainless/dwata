use anyhow::{Context, Result};
use clap::Parser;
use dwata_agents::email_entity_extractor::{parse_value, ExtractedEntitiesParams};
use dwata_agents::simple_email_content;
use dwata_agents::storage::{AgentStorage, InMemoryAgentStorage, Session};
use dwata_agents::EmailEntityExtractorAgent;
use dwata_api::database::emails as emails_db;
use dwata_api::helpers::database::initialize_database;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use std::sync::Arc;

#[derive(Debug, Parser)]
#[command(
    name = "extract_email_entities",
    about = "Extract all entities from a cleaned email using an LLM agent"
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

    let simple = simple_email_content(
        email.subject.as_deref(),
        email.body_text.as_deref(),
        email.body_html.as_deref(),
    );

    println!("Email ID: {}", email.id);
    println!("Subject: {}", simple.subject);
    println!();
    println!("--- Cleaned Body ---");
    println!("{}", simple.body);
    println!();
    println!("--- Running Entity Extraction Agent ---");
    println!();

    let llm_client = Arc::new(OllamaClient::new().context("Failed to initialize Ollama client")?);
    let storage = Arc::new(InMemoryAgentStorage::new());

    let session_id = storage
        .create_session(Session {
            id: None,
            agent_type: "email-entity-extractor".to_string(),
            objective: format!("Extract entities from email id={}", email.id),
            context_data: None,
            status: "running".to_string(),
            result: None,
        })
        .await
        .context("Failed to create agent session")?;

    let agent = EmailEntityExtractorAgent::new(
        llm_client,
        storage,
        MINISTRAL_3_3B_ID.to_string(),
        simple.subject.clone(),
        simple.body.clone(),
    );

    let entities = agent
        .execute(session_id)
        .await
        .context("Entity extraction failed")?;

    println!("--- Extracted Entities ---");
    println!();
    print_entities(&entities);

    Ok(())
}

fn print_entities(params: &ExtractedEntitiesParams) {
    if !params.locations.is_empty() {
        println!("## Locations");
        println!(
            "+{:-<4}+{:-<30}+{:-<20}+{:-<12}+{:-<30}+{:-<14}+",
            "", "", "", "", "", ""
        );
        println!(
            "| {:<2} | {:<28} | {:<18} | {:<10} | {:<28} | {:<12} |",
            "id", "city", "region", "country", "address_line1", "postal_code"
        );
        println!(
            "+{:-<4}+{:-<30}+{:-<20}+{:-<12}+{:-<30}+{:-<14}+",
            "", "", "", "", "", ""
        );
        for l in &params.locations {
            println!(
                "| {:<2} | {:<28} | {:<18} | {:<10} | {:<28} | {:<12} |",
                l.id,
                trunc(l.city.as_deref().unwrap_or(""), 28),
                trunc(l.region.as_deref().unwrap_or(""), 18),
                trunc(l.country_code.as_deref().unwrap_or(""), 10),
                trunc(l.address_line1.as_deref().unwrap_or(""), 28),
                trunc(l.postal_code.as_deref().unwrap_or(""), 12),
            );
        }
        println!(
            "+{:-<4}+{:-<30}+{:-<20}+{:-<12}+{:-<30}+{:-<14}+",
            "", "", "", "", "", ""
        );
        println!();
    }

    if !params.companies.is_empty() {
        println!("## Companies");
        println!(
            "+{:-<4}+{:-<35}+{:-<25}+{:-<40}+{:-<12}+",
            "", "", "", "", ""
        );
        println!(
            "| {:<2} | {:<33} | {:<23} | {:<38} | {:<10} |",
            "id", "name", "industry", "website", "location_id"
        );
        println!(
            "+{:-<4}+{:-<35}+{:-<25}+{:-<40}+{:-<12}+",
            "", "", "", "", ""
        );
        for c in &params.companies {
            println!(
                "| {:<2} | {:<33} | {:<23} | {:<38} | {:<10} |",
                c.id,
                trunc(&c.name, 33),
                trunc(c.industry.as_deref().unwrap_or(""), 23),
                trunc(c.website.as_deref().unwrap_or(""), 38),
                c.location_id.map(|v| v.to_string()).unwrap_or_default(),
            );
        }
        println!(
            "+{:-<4}+{:-<35}+{:-<25}+{:-<40}+{:-<12}+",
            "", "", "", "", ""
        );
        println!();
    }

    if !params.contacts.is_empty() {
        println!("## Contacts");
        println!(
            "+{:-<4}+{:-<30}+{:-<35}+{:-<18}+{:-<12}+",
            "", "", "", "", ""
        );
        println!(
            "| {:<2} | {:<28} | {:<33} | {:<16} | {:<10} |",
            "id", "name", "email", "phone", "company_id"
        );
        println!(
            "+{:-<4}+{:-<30}+{:-<35}+{:-<18}+{:-<12}+",
            "", "", "", "", ""
        );
        for c in &params.contacts {
            println!(
                "| {:<2} | {:<28} | {:<33} | {:<16} | {:<10} |",
                c.id,
                trunc(&c.name, 28),
                trunc(c.email.as_deref().unwrap_or(""), 33),
                trunc(c.phone.as_deref().unwrap_or(""), 16),
                c.company_id.map(|v| v.to_string()).unwrap_or_default(),
            );
        }
        println!(
            "+{:-<4}+{:-<30}+{:-<35}+{:-<18}+{:-<12}+",
            "", "", "", "", ""
        );
        println!();
    }

    if !params.vendors.is_empty() {
        println!("## Vendors");
        println!("+{:-<4}+{:-<35}+{:-<20}+{:-<30}+", "", "", "", "");
        println!(
            "| {:<2} | {:<33} | {:<18} | {:<28} |",
            "id", "vendor_name", "vendor_type", "vendor_external_id"
        );
        println!("+{:-<4}+{:-<35}+{:-<20}+{:-<30}+", "", "", "", "");
        for v in &params.vendors {
            println!(
                "| {:<2} | {:<33} | {:<18} | {:<28} |",
                v.id,
                trunc(&v.vendor_name, 33),
                trunc(&v.vendor_type, 18),
                trunc(v.vendor_external_id.as_deref().unwrap_or(""), 28),
            );
        }
        println!("+{:-<4}+{:-<35}+{:-<20}+{:-<30}+", "", "", "", "");
        println!();
    }

    if !params.bills.is_empty() {
        println!("## Bills");
        println!(
            "+{:-<4}+{:-<22}+{:-<20}+{:-<10}+{:-<30}+{:-<30}+{:-<12}+",
            "", "", "", "", "", "", ""
        );
        println!(
            "| {:<2} | {:<20} | {:<18} | {:<8} | {:<28} | {:<28} | {:<10} |",
            "id",
            "doc_type",
            "total_amount",
            "currency",
            "issued_date (parsed)",
            "due_date (parsed)",
            "vendor_id"
        );
        println!(
            "+{:-<4}+{:-<22}+{:-<20}+{:-<10}+{:-<30}+{:-<30}+{:-<12}+",
            "", "", "", "", "", "", ""
        );
        for b in &params.bills {
            let amount = b
                .total_amount
                .as_deref()
                .map(|v| parse_value(v).to_string())
                .unwrap_or_default();
            let issued = b
                .issued_date
                .as_deref()
                .map(|v| parse_value(v).to_string())
                .unwrap_or_default();
            let due = b
                .due_date
                .as_deref()
                .map(|v| parse_value(v).to_string())
                .unwrap_or_default();
            println!(
                "| {:<2} | {:<20} | {:<18} | {:<8} | {:<28} | {:<28} | {:<10} |",
                b.id,
                trunc(b.document_type.as_deref().unwrap_or(""), 20),
                trunc(&amount, 18),
                trunc(b.currency.as_deref().unwrap_or(""), 8),
                trunc(&issued, 28),
                trunc(&due, 28),
                b.issuer_vendor_id
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            );
        }
        println!(
            "+{:-<4}+{:-<22}+{:-<20}+{:-<10}+{:-<30}+{:-<30}+{:-<12}+",
            "", "", "", "", "", "", ""
        );
        println!();
    }

    if !params.transactions.is_empty() {
        println!("## Transactions");
        println!(
            "+{:-<4}+{:-<20}+{:-<10}+{:-<30}+{:-<30}+{:-<12}+{:-<12}+",
            "", "", "", "", "", "", ""
        );
        println!(
            "| {:<2} | {:<18} | {:<8} | {:<28} | {:<28} | {:<10} | {:<10} |",
            "id",
            "amount",
            "currency",
            "transaction_date (parsed)",
            "reference",
            "payer_id",
            "payee_id"
        );
        println!(
            "+{:-<4}+{:-<20}+{:-<10}+{:-<30}+{:-<30}+{:-<12}+{:-<12}+",
            "", "", "", "", "", "", ""
        );
        for t in &params.transactions {
            let amount = parse_value(&t.amount).to_string();
            let date = t
                .transaction_date
                .as_deref()
                .map(|v| parse_value(v).to_string())
                .unwrap_or_default();
            println!(
                "| {:<2} | {:<18} | {:<8} | {:<28} | {:<28} | {:<10} | {:<10} |",
                t.id,
                trunc(&amount, 18),
                trunc(&t.currency, 8),
                trunc(&date, 28),
                trunc(t.transaction_reference.as_deref().unwrap_or(""), 28),
                t.payer_vendor_id.map(|v| v.to_string()).unwrap_or_default(),
                t.payee_vendor_id.map(|v| v.to_string()).unwrap_or_default(),
            );
        }
        println!(
            "+{:-<4}+{:-<20}+{:-<10}+{:-<30}+{:-<30}+{:-<12}+{:-<12}+",
            "", "", "", "", "", "", ""
        );
        println!();
    }

    if !params.events.is_empty() {
        println!("## Events");
        println!(
            "+{:-<4}+{:-<35}+{:-<30}+{:-<40}+{:-<12}+",
            "", "", "", "", ""
        );
        println!(
            "| {:<2} | {:<33} | {:<28} | {:<38} | {:<10} |",
            "id", "name", "event_date (parsed)", "attendees", "location_id"
        );
        println!(
            "+{:-<4}+{:-<35}+{:-<30}+{:-<40}+{:-<12}+",
            "", "", "", "", ""
        );
        for e in &params.events {
            let date = e
                .event_date
                .as_deref()
                .map(|v| parse_value(v).to_string())
                .unwrap_or_default();
            println!(
                "| {:<2} | {:<33} | {:<28} | {:<38} | {:<10} |",
                e.id,
                trunc(&e.name, 33),
                trunc(&date, 28),
                trunc(&e.attendees.join(", "), 38),
                e.location_id.map(|v| v.to_string()).unwrap_or_default(),
            );
        }
        println!(
            "+{:-<4}+{:-<35}+{:-<30}+{:-<40}+{:-<12}+",
            "", "", "", "", ""
        );
        println!();
    }

    if params.locations.is_empty()
        && params.companies.is_empty()
        && params.contacts.is_empty()
        && params.vendors.is_empty()
        && params.bills.is_empty()
        && params.transactions.is_empty()
        && params.events.is_empty()
    {
        println!("(no entities extracted)");
    }
}

fn trunc(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
