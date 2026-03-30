use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use dwata_agents::{
    simple_email_content,
    storage::{AgentStorage, InMemoryAgentStorage, Session},
    EntitySearchProvider, EntitySearchResult, ExtractedEntitiesParams, KgEmailExtractionAgent,
    KgPersistenceProvider, SearchEntitiesParams, TemplateDocumentLabelerAgent,
};
use dwata_api::database::emails as emails_db;
use dwata_api::helpers::database::initialize_database;
use dwata_api::search::entity_index::{
    open_or_create_index, reindex_all_entities, DbEntitySearchProvider,
};
use nocodo_llm_sdk::llama_cpp::LlamaCppClient;
use nocodo_llm_sdk::models::llama_cpp::QWEN_3_5_0_8B;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "extract_kg_entities",
    about = "Run the 4-pass KG extraction pipeline against an email (display only, no DB writes)"
)]
struct Args {
    /// Email ID from the emails table
    email_id: i64,

    /// Skip document labeler and run all four passes unconditionally
    #[arg(long, default_value_t = false)]
    all_passes: bool,

    /// Base URL for llama.cpp OpenAI-compatible server
    #[arg(long, default_value = "http://localhost:8080")]
    llama_base_url: String,
}

// ---------------------------------------------------------------------------
// Simple ASCII table printer
// ---------------------------------------------------------------------------

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("  (none)");
        return;
    }

    // Compute column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let separator: String = widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("─┼─");
    let top_sep: String = widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("─┬─");
    let bot_sep: String = widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("─┴─");

    println!("  ┌─{}─┐", top_sep);
    let header_cells: String = headers
        .iter()
        .zip(widths.iter())
        .map(|(h, w)| format!(" {:width$} ", h, width = w))
        .collect::<Vec<_>>()
        .join("│");
    println!("  │{}│", header_cells);
    println!("  ├─{}─┤", separator);

    for row in rows {
        let cells: String = row
            .iter()
            .zip(widths.iter())
            .map(|(c, w)| format!(" {:width$} ", c, width = w))
            .collect::<Vec<_>>()
            .join("│");
        println!("  │{}│", cells);
    }

    println!("  └─{}─┘", bot_sep);
}

// ---------------------------------------------------------------------------
// Persistence provider: display only, no DB writes
// ---------------------------------------------------------------------------

struct StdoutPersistenceProvider;

#[async_trait]
impl KgPersistenceProvider for StdoutPersistenceProvider {
    async fn persist_pass_result(
        &self,
        params: &ExtractedEntitiesParams,
        _source_email_id: Option<i64>,
        _sender_email: Option<&str>,
    ) -> anyhow::Result<()> {
        println!();
        println!("  ── Extracted entities ──────────────────────────────────────────");

        if let Some(locs) = &params.locations {
            if !locs.is_empty() {
                println!();
                println!("  Locations ({}):", locs.len());
                let rows: Vec<Vec<String>> = locs
                    .iter()
                    .map(|l| {
                        vec![
                            l.id.to_string(),
                            l.name.clone().unwrap_or_default(),
                            l.city.clone().unwrap_or_default(),
                            l.region.clone().unwrap_or_default(),
                            l.country_code.clone().unwrap_or_default(),
                            l.postal_code.clone().unwrap_or_default(),
                            truncate(l.search_summary.as_deref().unwrap_or(""), 40),
                        ]
                    })
                    .collect();
                print_table(
                    &[
                        "ID", "Name", "City", "Region", "Country", "Postal", "Summary",
                    ],
                    &rows,
                );
            }
        }

        if let Some(orgs) = &params.organisations {
            if !orgs.is_empty() {
                println!();
                println!("  Organisations ({}):", orgs.len());
                let rows: Vec<Vec<String>> = orgs
                    .iter()
                    .map(|o| {
                        vec![
                            o.id.to_string(),
                            o.name.clone(),
                            o.industry.clone().unwrap_or_default(),
                            o.website.clone().unwrap_or_default(),
                            o.email.clone().unwrap_or_default(),
                            truncate(o.search_summary.as_deref().unwrap_or(""), 40),
                        ]
                    })
                    .collect();
                print_table(
                    &["ID", "Name", "Industry", "Website", "Email", "Summary"],
                    &rows,
                );
            }
        }

        if let Some(persons) = &params.persons {
            if !persons.is_empty() {
                println!();
                println!("  Persons ({}):", persons.len());
                let rows: Vec<Vec<String>> = persons
                    .iter()
                    .map(|p| {
                        vec![
                            p.id.to_string(),
                            p.name.clone(),
                            p.email.clone().unwrap_or_default(),
                            p.phone.clone().unwrap_or_default(),
                            p.organisation_id
                                .map(|id| id.to_string())
                                .unwrap_or_default(),
                            truncate(p.search_summary.as_deref().unwrap_or(""), 40),
                        ]
                    })
                    .collect();
                print_table(
                    &["ID", "Name", "Email", "Phone", "Org ID", "Summary"],
                    &rows,
                );
            }
        }

        if let Some(bills) = &params.bills {
            if !bills.is_empty() {
                println!();
                println!("  Bills ({}):", bills.len());
                let rows: Vec<Vec<String>> = bills
                    .iter()
                    .map(|b| {
                        vec![
                            b.id.to_string(),
                            b.total_amount.clone().unwrap_or_default(),
                            b.currency.clone().unwrap_or_default(),
                            b.issued_date.clone().unwrap_or_default(),
                            b.due_date.clone().unwrap_or_default(),
                            b.document_reference.clone().unwrap_or_default(),
                            b.issuer_organisation_id
                                .map(|id| id.to_string())
                                .unwrap_or_default(),
                        ]
                    })
                    .collect();
                print_table(
                    &[
                        "ID",
                        "Amount",
                        "Currency",
                        "Issued",
                        "Due",
                        "Ref",
                        "Issuer Org",
                    ],
                    &rows,
                );
            }
        }

        if let Some(txns) = &params.transactions {
            if !txns.is_empty() {
                println!();
                println!("  Transactions ({}):", txns.len());
                let rows: Vec<Vec<String>> = txns
                    .iter()
                    .map(|t| {
                        vec![
                            t.id.to_string(),
                            t.amount.to_string(),
                            t.currency.clone(),
                            t.transaction_date.clone().unwrap_or_default(),
                            t.transaction_reference.clone().unwrap_or_default(),
                            t.payer_organisation_id
                                .map(|id| id.to_string())
                                .unwrap_or_default(),
                            t.payee_organisation_id
                                .map(|id| id.to_string())
                                .unwrap_or_default(),
                        ]
                    })
                    .collect();
                print_table(
                    &[
                        "ID",
                        "Amount",
                        "Currency",
                        "Date",
                        "Ref",
                        "Payer Org",
                        "Payee Org",
                    ],
                    &rows,
                );
            }
        }

        if let Some(subs) = &params.subscriptions {
            if !subs.is_empty() {
                println!();
                println!("  Subscriptions ({}):", subs.len());
                let rows: Vec<Vec<String>> = subs
                    .iter()
                    .map(|s| {
                        vec![
                            s.id.to_string(),
                            s.service_name.clone(),
                            s.plan_name.clone().unwrap_or_default(),
                            s.billing_cycle.clone().unwrap_or_default(),
                            s.amount.map(|a| a.to_string()).unwrap_or_default(),
                            s.currency.clone().unwrap_or_default(),
                            s.next_billing_date.clone().unwrap_or_default(),
                        ]
                    })
                    .collect();
                print_table(
                    &[
                        "ID",
                        "Service",
                        "Plan",
                        "Cycle",
                        "Amount",
                        "Currency",
                        "Next Bill",
                    ],
                    &rows,
                );
            }
        }

        if let Some(orders) = &params.orders {
            if !orders.is_empty() {
                println!();
                println!("  Orders ({}):", orders.len());
                let rows: Vec<Vec<String>> = orders
                    .iter()
                    .map(|o| {
                        vec![
                            o.id.to_string(),
                            o.order_reference.clone().unwrap_or_default(),
                            o.order_date.clone().unwrap_or_default(),
                            o.status.clone().unwrap_or_default(),
                            o.total_amount.map(|a| a.to_string()).unwrap_or_default(),
                            o.currency.clone().unwrap_or_default(),
                            o.tracking_number.clone().unwrap_or_default(),
                        ]
                    })
                    .collect();
                print_table(
                    &[
                        "ID",
                        "Order Ref",
                        "Date",
                        "Status",
                        "Amount",
                        "Currency",
                        "Tracking",
                    ],
                    &rows,
                );
            }
        }

        if let Some(events) = &params.events {
            if !events.is_empty() {
                println!();
                println!("  Events ({}):", events.len());
                let rows: Vec<Vec<String>> = events
                    .iter()
                    .map(|e| {
                        vec![
                            e.id.to_string(),
                            e.name.clone(),
                            e.event_date.clone().unwrap_or_default(),
                            e.location_id.map(|id| id.to_string()).unwrap_or_default(),
                            e.attendees
                                .as_ref()
                                .map(|a| a.join(", "))
                                .unwrap_or_default(),
                        ]
                    })
                    .collect();
                print_table(&["ID", "Name", "Date", "Location ID", "Attendees"], &rows);
            }
        }

        let total = params.locations.as_ref().map_or(0, |v| v.len())
            + params.organisations.as_ref().map_or(0, |v| v.len())
            + params.persons.as_ref().map_or(0, |v| v.len())
            + params.bills.as_ref().map_or(0, |v| v.len())
            + params.transactions.as_ref().map_or(0, |v| v.len())
            + params.subscriptions.as_ref().map_or(0, |v| v.len())
            + params.orders.as_ref().map_or(0, |v| v.len())
            + params.events.as_ref().map_or(0, |v| v.len());

        if total == 0 {
            println!("  (no entities extracted)");
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Search provider wrapper: logs pre-population results before forwarding
// ---------------------------------------------------------------------------

struct LoggingSearchProvider {
    inner: Arc<DbEntitySearchProvider>,
}

impl LoggingSearchProvider {
    fn new(inner: Arc<DbEntitySearchProvider>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl EntitySearchProvider for LoggingSearchProvider {
    async fn search_entities(
        &self,
        params: &SearchEntitiesParams,
    ) -> anyhow::Result<Vec<EntitySearchResult>> {
        let results = self.inner.search_entities(params).await?;

        let types_str: String = params
            .entity_types
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!();
        println!(
            "  ── Pre-population search (keywords: {:?}, types: {}) ─────────",
            params.keywords, types_str
        );

        if results.is_empty() {
            println!("  (no existing entities found)");
        } else {
            let rows: Vec<Vec<String>> = results
                .iter()
                .map(|r| {
                    vec![
                        r.id.to_string(),
                        r.entity_type.as_str().to_string(),
                        r.name.clone(),
                        format!("{:.2}", r.score),
                        truncate(r.search_summary.as_deref().unwrap_or(""), 50),
                    ]
                })
                .collect();
            print_table(&["ID", "Type", "Name", "Score", "Summary"], &rows);
        }

        Ok(results)
    }
}

// ---------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("dwata_agents=info".parse()?))
        .with_target(false)
        .init();

    let args = Args::parse();
    let selected_model = QWEN_3_5_0_8B;

    let db = initialize_database().context("Failed to initialize database")?;

    let entity_index_path = dirs::data_local_dir()
        .map(|d| d.join("dwata").join("entity-index"))
        .context("Failed to resolve entity index path")?;

    let entity_index = open_or_create_index(&entity_index_path)
        .context("Failed to initialize entity search index")?;

    let pool = db.async_connection.pool();

    // Reindex all existing entities so pre-population works from the start.
    reindex_all_entities(&pool, &entity_index)
        .await
        .context("Failed to reindex entities")?;

    let llm_client = Arc::new(
        LlamaCppClient::new()
            .context("Failed to initialize llama.cpp client")?
            .with_base_url(args.llama_base_url.clone()),
    );
    let storage = Arc::new(InMemoryAgentStorage::new());

    let email = emails_db::get_email(db.async_connection.clone(), args.email_id)
        .await
        .with_context(|| format!("Failed to load email id={}", args.email_id))?;

    let simple = simple_email_content(
        email.subject.as_deref(),
        email.body_text.as_deref(),
        email.body_html.as_deref(),
    );

    println!("Email ID:  {}", email.id);
    println!("Subject:   {}", simple.subject);
    println!("Model:     {}", selected_model);
    println!("Provider:  llama.cpp ({})", args.llama_base_url);
    println!();

    // --- Step 1: Document labeling (skip if --all-passes) ---
    let label = if args.all_passes {
        println!("Skipping document labeler (--all-passes set).");
        None
    } else {
        println!("Running document labeler...");
        let label_session_id = storage
            .create_session(Session {
                id: None,
                agent_type: "document-labeler".to_string(),
                objective: format!("Label email id={}", email.id),
                context_data: None,
                status: "running".to_string(),
                result: None,
            })
            .await
            .context("Failed to create labeler session")?;

        let labeler = TemplateDocumentLabelerAgent::new(
            llm_client.clone(),
            storage.clone(),
            selected_model.to_string(),
            simple.body.clone(),
        );

        match labeler.execute(label_session_id).await {
            Ok(label) => {
                println!(
                    "Label: doc_type={:?}  has_bill={}  has_transaction={}  has_event={}  has_order={}",
                    label.doc_type,
                    label.has_bill,
                    label.has_transaction,
                    label.has_event,
                    label.has_order,
                );
                Some(label)
            }
            Err(e) => {
                tracing::warn!("Document labeler failed ({}), running all passes", e);
                None
            }
        }
    };

    println!();
    println!("--- Running KG extraction passes ---");
    println!();

    // Display-only persistence — entities are printed, not saved to DB.
    let persistence = Arc::new(StdoutPersistenceProvider);

    // Logging wrapper around the real search provider.
    let inner_search = Arc::new(DbEntitySearchProvider::new(pool.clone(), entity_index));
    let search_provider = Arc::new(LoggingSearchProvider::new(inner_search));

    let kg_session_id = storage
        .create_session(Session {
            id: None,
            agent_type: "kg-email-extractor".to_string(),
            objective: format!("KG extraction from email id={}", email.id),
            context_data: None,
            status: "running".to_string(),
            result: None,
        })
        .await
        .context("Failed to create KG session")?;

    let email_content = format!("Subject: {}\n\n{}", simple.subject, simple.body);

    let mut agent = KgEmailExtractionAgent::new(
        llm_client,
        storage,
        persistence,
        selected_model.to_string(),
        email_content,
    )
    .with_single_tool_submission(true)
    .with_search_provider(search_provider)
    .with_source_email_id(email.id)
    .with_sender(email.from_name.clone(), Some(email.from_address.clone()));

    if let Some(label) = label {
        agent = agent.with_label(label);
    }

    agent
        .execute(kg_session_id)
        .await
        .context("KG extraction failed")?;

    println!();
    println!("KG extraction complete (display only — nothing written to DB).");

    Ok(())
}
