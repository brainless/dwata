use anyhow::{Context, Result};
use clap::Parser;
use dwata_agents::email_entity_extractor::{parse_value, ExtractedEntitiesParams};
use dwata_agents::simple_email_content;
use dwata_agents::storage::{AgentStorage, InMemoryAgentStorage, Session};
use dwata_agents::EmailEntityExtractorAgent;
use dwata_api::database::emails as emails_db;
use dwata_api::helpers::database::initialize_database;
use dwata_api::helpers::email_search_provider::TantivyEmailSearchProvider;
use dwata_api::search::tantivy::open_or_create_index;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

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
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("dwata_agents=info".parse()?))
        .with_target(false)
        .init();

    let args = Args::parse();
    let db = initialize_database().context("Failed to initialize database")?;

    let search_index_path = dirs::data_local_dir()
        .map(|d| d.join("dwata").join("tantivy-index"))
        .context("Failed to resolve search index path")?;
    let search_index = Arc::new(
        open_or_create_index(&search_index_path).context("Failed to initialize search index")?,
    );

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

    let search_provider = Arc::new(TantivyEmailSearchProvider::new(
        search_index,
        db.async_connection.clone(),
        email.from_address.clone(),
    ));

    let agent = EmailEntityExtractorAgent::new(
        llm_client,
        storage,
        MINISTRAL_3_3B_ID.to_string(),
        simple.subject.clone(),
        simple.body.clone(),
        Some(search_provider),
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
    let mut any = false;

    if let Some(locations) = &params.locations {
        if !locations.is_empty() {
            any = true;
            println!("## Locations");
            print_sep(&[4, 30, 20, 12, 30, 14]);
            println!(
                "| {:<2} | {:<28} | {:<18} | {:<10} | {:<28} | {:<12} |",
                "id", "city", "region", "country", "address_line1", "postal_code"
            );
            print_sep(&[4, 30, 20, 12, 30, 14]);
            for l in locations {
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
            print_sep(&[4, 30, 20, 12, 30, 14]);
            println!();
        }
    }

    if let Some(organisations) = &params.organisations {
        if !organisations.is_empty() {
            any = true;
            println!("## Organisations");
            print_sep(&[4, 35, 20, 25, 40, 12]);
            println!(
                "| {:<2} | {:<33} | {:<18} | {:<23} | {:<38} | {:<10} |",
                "id", "name", "role", "industry", "website", "location_id"
            );
            print_sep(&[4, 35, 20, 25, 40, 12]);
            for o in organisations {
                let role_str = o.role.map(|r| r.to_string()).unwrap_or_default();
                println!(
                    "| {:<2} | {:<33} | {:<18} | {:<23} | {:<38} | {:<10} |",
                    o.id,
                    trunc(&o.name, 33),
                    trunc(&role_str, 18),
                    trunc(o.industry.as_deref().unwrap_or(""), 23),
                    trunc(o.website.as_deref().unwrap_or(""), 38),
                    o.location_id.map(|v| v.to_string()).unwrap_or_default(),
                );
            }
            print_sep(&[4, 35, 20, 25, 40, 12]);
            println!();
        }
    }

    if let Some(persons) = &params.persons {
        if !persons.is_empty() {
            any = true;
            println!("## Persons");
            print_sep(&[4, 30, 35, 18, 12]);
            println!(
                "| {:<2} | {:<28} | {:<33} | {:<16} | {:<10} |",
                "id", "name", "email", "phone", "org_id"
            );
            print_sep(&[4, 30, 35, 18, 12]);
            for p in persons {
                println!(
                    "| {:<2} | {:<28} | {:<33} | {:<16} | {:<10} |",
                    p.id,
                    trunc(&p.name, 28),
                    trunc(p.email.as_deref().unwrap_or(""), 33),
                    trunc(p.phone.as_deref().unwrap_or(""), 16),
                    p.organisation_id.map(|v| v.to_string()).unwrap_or_default(),
                );
            }
            print_sep(&[4, 30, 35, 18, 12]);
            println!();
        }
    }

    if let Some(bills) = &params.bills {
        if !bills.is_empty() {
            any = true;
            println!("## Bills");
            print_sep(&[4, 22, 20, 10, 30, 30, 12, 12]);
            println!(
                "| {:<2} | {:<20} | {:<18} | {:<8} | {:<28} | {:<28} | {:<10} | {:<10} |",
                "id",
                "doc_ref",
                "total_amount",
                "currency",
                "issued_date (parsed)",
                "due_date (parsed)",
                "org_id",
                "sub_id"
            );
            print_sep(&[4, 22, 20, 10, 30, 30, 12, 12]);
            for b in bills {
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
                    "| {:<2} | {:<20} | {:<18} | {:<8} | {:<28} | {:<28} | {:<10} | {:<10} |",
                    b.id,
                    trunc(b.document_reference.as_deref().unwrap_or(""), 20),
                    trunc(&amount, 18),
                    trunc(b.currency.as_deref().unwrap_or(""), 8),
                    trunc(&issued, 28),
                    trunc(&due, 28),
                    b.issuer_organisation_id
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    b.subscription_id.map(|v| v.to_string()).unwrap_or_default(),
                );
            }
            print_sep(&[4, 22, 20, 10, 30, 30, 12, 12]);
            println!();
        }
    }

    if let Some(transactions) = &params.transactions {
        if !transactions.is_empty() {
            any = true;
            println!("## Transactions");
            print_sep(&[4, 20, 10, 30, 30, 12, 12]);
            println!(
                "| {:<2} | {:<18} | {:<8} | {:<28} | {:<28} | {:<10} | {:<10} |",
                "id", "amount", "currency", "date (parsed)", "reference", "payer_id", "payee_id"
            );
            print_sep(&[4, 20, 10, 30, 30, 12, 12]);
            for t in transactions {
                let amount = format!("{:.2}", t.amount);
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
                    t.payer_organisation_id
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    t.payee_organisation_id
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                );
            }
            print_sep(&[4, 20, 10, 30, 30, 12, 12]);
            println!();
        }
    }

    if let Some(subscriptions) = &params.subscriptions {
        if !subscriptions.is_empty() {
            any = true;
            println!("## Subscriptions");
            print_sep(&[4, 30, 25, 15, 20, 10, 12]);
            println!(
                "| {:<2} | {:<28} | {:<23} | {:<13} | {:<18} | {:<8} | {:<10} |",
                "id",
                "service_name",
                "plan_name",
                "billing_cycle",
                "amount (parsed)",
                "currency",
                "org_id"
            );
            print_sep(&[4, 30, 25, 15, 20, 10, 12]);
            for s in subscriptions {
                let amount = s.amount.map(|v| format!("{:.2}", v)).unwrap_or_default();
                let next = s
                    .next_billing_date
                    .as_deref()
                    .map(|v| parse_value(v).to_string())
                    .unwrap_or_default();
                println!(
                    "| {:<2} | {:<28} | {:<23} | {:<13} | {:<18} | {:<8} | {:<10} |",
                    s.id,
                    trunc(&s.service_name, 28),
                    trunc(s.plan_name.as_deref().unwrap_or(""), 23),
                    trunc(s.billing_cycle.as_deref().unwrap_or(""), 13),
                    trunc(&amount, 18),
                    trunc(s.currency.as_deref().unwrap_or(""), 8),
                    s.organisation_id.map(|v| v.to_string()).unwrap_or_default(),
                );
                if !next.is_empty() {
                    println!("       next_billing_date: {}", next);
                }
            }
            print_sep(&[4, 30, 25, 15, 20, 10, 12]);
            println!();
        }
    }

    if let Some(orders) = &params.orders {
        if !orders.is_empty() {
            any = true;
            println!("## Orders");
            print_sep(&[4, 25, 20, 20, 10, 30, 12]);
            println!(
                "| {:<2} | {:<23} | {:<18} | {:<18} | {:<8} | {:<28} | {:<10} |",
                "id",
                "order_reference",
                "status",
                "total_amount",
                "currency",
                "tracking_number",
                "org_id"
            );
            print_sep(&[4, 25, 20, 20, 10, 30, 12]);
            for o in orders {
                let amount = o
                    .total_amount
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_default();
                println!(
                    "| {:<2} | {:<23} | {:<18} | {:<18} | {:<8} | {:<28} | {:<10} |",
                    o.id,
                    trunc(o.order_reference.as_deref().unwrap_or(""), 23),
                    trunc(o.status.as_deref().unwrap_or(""), 18),
                    trunc(&amount, 18),
                    trunc(o.currency.as_deref().unwrap_or(""), 8),
                    trunc(o.tracking_number.as_deref().unwrap_or(""), 28),
                    o.organisation_id.map(|v| v.to_string()).unwrap_or_default(),
                );
                if let Some(items) = &o.items {
                    if !items.is_empty() {
                        println!("       items: {}", items.join(", "));
                    }
                }
            }
            print_sep(&[4, 25, 20, 20, 10, 30, 12]);
            println!();
        }
    }

    if let Some(events) = &params.events {
        if !events.is_empty() {
            any = true;
            println!("## Events");
            print_sep(&[4, 35, 30, 40, 12]);
            println!(
                "| {:<2} | {:<33} | {:<28} | {:<38} | {:<10} |",
                "id", "name", "event_date (parsed)", "attendees", "location_id"
            );
            print_sep(&[4, 35, 30, 40, 12]);
            for e in events {
                let date = e
                    .event_date
                    .as_deref()
                    .map(|v| parse_value(v).to_string())
                    .unwrap_or_default();
                let attendees = e.attendees.as_deref().unwrap_or(&[]).join(", ");
                println!(
                    "| {:<2} | {:<33} | {:<28} | {:<38} | {:<10} |",
                    e.id,
                    trunc(&e.name, 33),
                    trunc(&date, 28),
                    trunc(&attendees, 38),
                    e.location_id.map(|v| v.to_string()).unwrap_or_default(),
                );
            }
            print_sep(&[4, 35, 30, 40, 12]);
            println!();
        }
    }

    if !any {
        println!("(no entities extracted)");
    }
}

fn print_sep(widths: &[usize]) {
    let parts: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    println!("+{}+", parts.join("+"));
}

fn trunc(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
