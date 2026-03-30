use anyhow::{Context, Result};
use clap::Parser;
use dwata_agents::{
    simple_email_content,
    storage::{AgentStorage, InMemoryAgentStorage, Session},
    KgEmailExtractionAgent, TemplateDocumentLabelerAgent,
};
use dwata_api::database::emails as emails_db;
use dwata_api::database::kg_entities::KgPersistenceLayer;
use dwata_api::helpers::database::initialize_database;
use dwata_api::search::entity_index::{
    open_or_create_index, reindex_all_entities, DbEntitySearchProvider,
};
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "extract_kg_entities",
    about = "Run the 4-pass KG extraction pipeline against an email"
)]
struct Args {
    /// Email ID from the emails table
    email_id: i64,

    /// Skip document labeler and run all four passes unconditionally
    #[arg(long, default_value_t = false)]
    all_passes: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("dwata_agents=info".parse()?))
        .with_target(false)
        .init();

    let args = Args::parse();

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

    let llm_client = Arc::new(OllamaClient::new().context("Failed to initialize Ollama client")?);
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
            MINISTRAL_3_3B_ID.to_string(),
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

    // --- Step 2: KG extraction ---
    let persistence = Arc::new(KgPersistenceLayer::new(
        pool.clone(),
        Some(entity_index.clone()),
    ));

    let search_provider = Arc::new(DbEntitySearchProvider::new(pool.clone(), entity_index));

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
        MINISTRAL_3_3B_ID.to_string(),
        email_content,
    )
    .with_search_provider(search_provider)
    .with_source_email_id(email.id);

    if let Some(label) = label {
        agent = agent.with_label(label);
    }

    agent
        .execute(kg_session_id)
        .await
        .context("KG extraction failed")?;

    println!("KG extraction complete.");

    Ok(())
}
