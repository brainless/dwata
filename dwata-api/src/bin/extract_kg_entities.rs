use anyhow::{Context, Result};
use clap::Parser;
use dwata_agents::{
    simple_email_content,
    storage::{AgentStorage, InMemoryAgentStorage, Session},
    ExtractionStateProvider, ExtractionStep, ExtractionStepState, InMemoryExtractionState,
    KgEmailExtractionAgent, TemplateDocumentLabelerAgent,
};
use dwata_api::database::emails as emails_db;
use dwata_api::helpers::database::initialize_database;
use dwata_api::search::entity_index::{
    open_or_create_index, reindex_all_entities, DbEntitySearchProvider,
};
use futures::executor::block_on;
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

    /// Show detailed step-by-step extraction progress
    #[arg(long, default_value_t = false)]
    verbose: bool,
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
// CLI Extraction State Reporter - prints step events to stdout
// ---------------------------------------------------------------------------

struct CliExtractionReporter;

impl CliExtractionReporter {
    fn new() -> Self {
        Self
    }

    fn print_step(&self, step: &ExtractionStep) {
        match step {
            ExtractionStep::DocumentLabeled { label, .. } => {
                println!();
                println!("  ── Document labeled ──────────────────────────────────────────");
                println!(
                    "  doc_type={:?}  has_bill={}  has_transaction={}  has_event={}  has_order={}",
                    label.doc_type,
                    label.has_bill,
                    label.has_transaction,
                    label.has_event,
                    label.has_order,
                );
            }
            ExtractionStep::PassStarted { pass_name, .. } => {
                println!();
                println!(
                    "  ── Starting pass: {} ──────────────────────────────────────────",
                    pass_name
                );
            }
            ExtractionStep::SearchPerformed {
                keywords,
                entity_types,
                results,
                result_count,
                ..
            } => {
                if !results.is_empty() {
                    println!();
                    let types_str = entity_types.join(", ");
                    println!(
                        "  ── Pre-population search (keywords: {:?}, types: {}) ─────────",
                        keywords, types_str
                    );
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
            }
            ExtractionStep::SenderSearchPerformed {
                sender_email,
                results,
                ..
            } => {
                if !results.is_empty() {
                    println!();
                    println!(
                        "  ── Sender search (email: {}) ─────────────────────────────────",
                        sender_email
                    );
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
            }
            ExtractionStep::EntitiesExtracted {
                entity_counts,
                total_entities,
                ..
            } => {
                if *total_entities > 0 {
                    println!();
                    println!("  ── Extracted entities ────────────────────────────────────────");
                    for (entity_type, count) in entity_counts {
                        println!("  {}: {}", entity_type, count);
                    }
                    println!("  Total: {} entities", total_entities);
                }
            }
            ExtractionStep::PassCompleted { .. } => {
                println!("  ✓ Pass completed");
            }
            ExtractionStep::PassFailed { error_message, .. } => {
                println!();
                println!("  ✗ Pass failed: {}", error_message);
            }
            ExtractionStep::ToolCallMade { tool_name, .. } => {
                println!("  → Tool called: {}", tool_name);
            }
            ExtractionStep::RetryOccurred {
                reason,
                attempt,
                max_attempts,
                ..
            } => {
                let reason_str = match reason {
                    dwata_agents::RetryReason::ParseFailed => "parse failed",
                    dwata_agents::RetryReason::ConfirmBeforeSubmit => "confirm before submit",
                    dwata_agents::RetryReason::EmptyConfirm => "empty confirm",
                    dwata_agents::RetryReason::NoToolCalls => "no tool calls",
                };
                println!("  ⚠ Retry {}/{} ({})", attempt, max_attempts, reason_str);
            }
        }
    }

    fn print_summary(&self, state: &ExtractionStepState) {
        println!();
        println!("═══ Extraction Summary ═══");
        println!("  Session ID: {}", state.session_id);
        println!("  Email ID: {:?}", state.summary.email_id);
        println!("  Status: {}", state.summary.status.as_str());
        println!("  Total passes: {}", state.summary.total_passes);
        println!("  Completed passes: {}", state.summary.completed_passes);
        println!("  Failed passes: {}", state.summary.failed_passes);
        println!(
            "  Total entities extracted: {}",
            state.summary.total_entities_extracted
        );
        println!(
            "  Total search results: {}",
            state.summary.total_search_results
        );
        if let Some(ref error) = state.summary.error_message {
            println!("  Error: {}", error);
        }
    }
}

// ---------------------------------------------------------------------------
// CLI State Provider - wraps InMemoryExtractionState with printing
// ---------------------------------------------------------------------------

struct CliExtractionStateProvider {
    inner: InMemoryExtractionState,
    reporter: CliExtractionReporter,
    verbose: bool,
}

impl CliExtractionStateProvider {
    fn new(verbose: bool) -> Self {
        Self {
            inner: InMemoryExtractionState::new(),
            reporter: CliExtractionReporter::new(),
            verbose,
        }
    }

    fn get_state(&self, session_id: i64) -> Option<ExtractionStepState> {
        // Use blocking approach for CLI since we're in a non-async context here
        // but the trait requires async
        block_on(self.inner.get_state(session_id))
    }

    fn print_final_summary(&self, session_id: i64) {
        if let Some(state) = self.get_state(session_id) {
            self.reporter.print_summary(&state);
        }
    }
}

#[async_trait::async_trait]
impl ExtractionStateProvider for CliExtractionStateProvider {
    async fn record_step(&self, session_id: i64, step: ExtractionStep) {
        if self.verbose {
            self.reporter.print_step(&step);
        }
        self.inner.record_step(session_id, step).await;
    }

    async fn get_state(&self, session_id: i64) -> Option<ExtractionStepState> {
        self.inner.get_state(session_id).await
    }

    async fn initialize_state(
        &self,
        session_id: i64,
        email_id: Option<i64>,
        sender_email: Option<String>,
    ) {
        self.inner
            .initialize_state(session_id, email_id, sender_email)
            .await;
    }

    async fn complete_extraction(&self, session_id: i64) {
        self.inner.complete_extraction(session_id).await;
    }

    async fn fail_extraction(&self, session_id: i64, error_message: String) {
        self.inner.fail_extraction(session_id, error_message).await;
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

    // Create extraction state provider
    let state_provider = Arc::new(CliExtractionStateProvider::new(args.verbose));

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
                // Record the document labeled step
                state_provider
                    .record_step(
                        label_session_id,
                        ExtractionStep::DocumentLabeled {
                            timestamp: current_timestamp(),
                            label: label.clone(),
                        },
                    )
                    .await;

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

    let inner_search = Arc::new(DbEntitySearchProvider::new(pool.clone(), entity_index));

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
        state_provider.clone(),
        selected_model.to_string(),
        email_content,
    )
    .with_extraction_state(state_provider.clone())
    .with_single_tool_submission(true)
    .with_search_provider(inner_search)
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

    // Print final summary if verbose mode is enabled
    if args.verbose {
        state_provider.print_final_summary(kg_session_id);
    }

    Ok(())
}

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
