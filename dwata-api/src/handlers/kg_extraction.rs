use actix_web::{web, HttpResponse, Result};
use std::sync::Arc;

use crate::database::credentials as credentials_db;
use crate::database::emails as emails_db;
use crate::database::kg_entities::KgPersistenceLayer;
use crate::database::kg_extraction;
use crate::database::Database;
use crate::search::entity_index::{
    open_or_create_index, reindex_all_entities, DbEntitySearchProvider,
};
use crate::state::kg_extraction::{AccountProgress, ExtractionStatus, KgExtractionState};
use dwata_agents::{
    simple_email_content,
    storage::{AgentStorage, InMemoryAgentStorage, Session},
    KgEmailExtractionAgent, TemplateDocumentLabelerAgent,
};
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use serde::Deserialize;

const DEFAULT_BATCH_SIZE: usize = 100;
const LONG_POLL_TIMEOUT_SECS: u64 = 30; // Maximum time to hold connection open

#[derive(Debug, Deserialize)]
pub struct RunKgExtractionRequest {
    /// Optional: specific credential_id to process (if None, processes all)
    pub credential_id: Option<i64>,
    /// Optional: batch size override
    pub batch_size: Option<usize>,
    /// Optional: skip document labeler and run all passes
    pub all_passes: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ProgressQueryParams {
    /// If true, enable long polling (wait for updates)
    pub long_poll: Option<bool>,
    /// Optional: specific credential_id to get progress for
    pub credential_id: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct KgExtractionResponse {
    pub success: bool,
    pub message: String,
    pub accounts_processed: Vec<AccountResult>,
    pub total_emails_processed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct AccountResult {
    pub credential_id: i64,
    pub identifier: String,
    pub emails_processed: usize,
    pub total_emails: i64,
    pub unprocessed_remaining: i64,
    pub success: bool,
    pub error: Option<String>,
}

/// Response type for progress endpoint
#[derive(Debug, serde::Serialize)]
pub struct ProgressResponse {
    pub active: bool,
    pub accounts: Vec<AccountProgressJson>,
    pub updated: bool, // True if this response contains new data (long poll)
}

#[derive(Debug, serde::Serialize)]
pub struct AccountProgressJson {
    pub credential_id: i64,
    pub identifier: String,
    pub status: String,
    pub total_emails: i64,
    pub emails_processed: usize,
    pub emails_failed: usize,
    pub current_email_id: Option<i64>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
}

impl From<AccountProgress> for AccountProgressJson {
    fn from(p: AccountProgress) -> Self {
        Self {
            credential_id: p.credential_id,
            identifier: p.identifier,
            status: p.status.as_str().to_string(),
            total_emails: p.total_emails,
            emails_processed: p.emails_processed,
            emails_failed: p.emails_failed,
            current_email_id: p.current_email_id,
            started_at: p.started_at,
            completed_at: p.completed_at,
            error_message: p.error_message,
        }
    }
}

/// Get current extraction progress with optional long polling
///
/// Long polling: When long_poll=true, the server holds the connection
/// open until progress updates or timeout (30s), then returns immediately.
/// Client should reconnect immediately to maintain real-time updates.
pub async fn get_kg_extraction_progress(
    state: web::Data<Arc<KgExtractionState>>,
    query: web::Query<ProgressQueryParams>,
) -> Result<HttpResponse> {
    let long_poll = query.long_poll.unwrap_or(false);

    if long_poll {
        // Long polling: wait for updates or timeout
        let updated = state.wait_for_updates(LONG_POLL_TIMEOUT_SECS).await;

        let progress = if let Some(cred_id) = query.credential_id {
            state
                .get_account_progress(cred_id)
                .map(|p| vec![p])
                .unwrap_or_default()
        } else {
            state.get_all_progress()
        };

        let accounts: Vec<AccountProgressJson> = progress.into_iter().map(|p| p.into()).collect();

        Ok(HttpResponse::Ok().json(ProgressResponse {
            active: state.is_active(),
            accounts,
            updated,
        }))
    } else {
        // Immediate response
        let progress = if let Some(cred_id) = query.credential_id {
            state
                .get_account_progress(cred_id)
                .map(|p| vec![p])
                .unwrap_or_default()
        } else {
            state.get_all_progress()
        };

        let accounts: Vec<AccountProgressJson> = progress.into_iter().map(|p| p.into()).collect();

        Ok(HttpResponse::Ok().json(ProgressResponse {
            active: state.is_active(),
            accounts,
            updated: false,
        }))
    }
}

/// Run KG extraction for all email accounts or a specific account
/// Processes emails in batches, most recent first
pub async fn run_kg_extraction(
    db: web::Data<Arc<Database>>,
    state: web::Data<Arc<KgExtractionState>>,
    request: web::Json<RunKgExtractionRequest>,
) -> Result<HttpResponse> {
    let batch_size = request.batch_size.unwrap_or(DEFAULT_BATCH_SIZE);
    let all_passes = request.all_passes.unwrap_or(false);

    // Get credentials to process
    let credentials = if let Some(cred_id) = request.credential_id {
        match credentials_db::get_credential(db.async_connection.clone(), cred_id).await {
            Ok(cred) => vec![cred],
            Err(_) => {
                return Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "error": format!("Credential {} not found", cred_id)
                })));
            }
        }
    } else {
        credentials_db::list_credentials(db.async_connection.clone(), false)
            .await
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Failed to list credentials: {}",
                    e
                ))
            })?
    };

    if credentials.is_empty() {
        return Ok(HttpResponse::Ok().json(KgExtractionResponse {
            success: true,
            message: "No credentials found to process".to_string(),
            accounts_processed: vec![],
            total_emails_processed: 0,
            errors: vec![],
        }));
    }

    // Initialize entity index (similar to CLI)
    let entity_index_path = dirs::data_local_dir()
        .map(|d| d.join("dwata").join("entity-index"))
        .ok_or_else(|| {
            actix_web::error::ErrorInternalServerError("Failed to resolve entity index path")
        })?;

    let entity_index = open_or_create_index(&entity_index_path).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to initialize entity search index: {}",
            e
        ))
    })?;

    let pool = db.async_connection.pool();

    // Reindex all existing entities so pre-population works
    reindex_all_entities(&pool, &entity_index)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to reindex entities: {}", e))
        })?;

    // Initialize LLM client
    let llm_client = Arc::new(OllamaClient::new().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to initialize Ollama client: {}",
            e
        ))
    })?);

    let storage = Arc::new(InMemoryAgentStorage::new());
    let persistence = Arc::new(KgPersistenceLayer::new(
        pool.clone(),
        Some(entity_index.clone()),
    ));
    let search_provider = Arc::new(DbEntitySearchProvider::new(pool.clone(), entity_index));

    let mut accounts_processed = Vec::new();
    let mut total_emails_processed = 0_usize;
    let mut global_errors = Vec::new();

    // Process each credential
    for credential in credentials {
        let credential_id = credential.id;
        let identifier = credential.identifier.clone();

        tracing::info!("Processing credential {} ({})", credential_id, identifier);

        // Count total and unprocessed emails
        let total_count =
            match kg_extraction::count_total_emails(db.async_connection.clone(), credential_id)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    let error_msg = format!(
                        "Failed to count total emails for credential {}: {}",
                        credential_id, e
                    );
                    tracing::error!("{}", error_msg);
                    state.fail_account(credential_id, error_msg.clone());
                    accounts_processed.push(AccountResult {
                        credential_id,
                        identifier: identifier.clone(),
                        emails_processed: 0,
                        total_emails: 0,
                        unprocessed_remaining: 0,
                        success: false,
                        error: Some(error_msg.clone()),
                    });
                    global_errors.push(error_msg);
                    continue;
                }
            };

        // Initialize state for this account
        state.start_account(credential_id, identifier.clone(), total_count);

        let unprocessed_count = match kg_extraction::count_unprocessed_emails(
            db.async_connection.clone(),
            credential_id,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                let error_msg = format!(
                    "Failed to count unprocessed emails for credential {}: {}",
                    credential_id, e
                );
                tracing::error!("{}", error_msg);
                state.fail_account(credential_id, error_msg.clone());
                accounts_processed.push(AccountResult {
                    credential_id,
                    identifier: identifier.clone(),
                    emails_processed: 0,
                    total_emails: total_count,
                    unprocessed_remaining: 0,
                    success: false,
                    error: Some(error_msg.clone()),
                });
                global_errors.push(error_msg);
                continue;
            }
        };

        if unprocessed_count == 0 {
            tracing::info!("No unprocessed emails for credential {}", credential_id);
            state.complete_account(credential_id);
            accounts_processed.push(AccountResult {
                credential_id,
                identifier: identifier.clone(),
                emails_processed: 0,
                total_emails: total_count,
                unprocessed_remaining: 0,
                success: true,
                error: None,
            });
            continue;
        }

        // Get unprocessed email IDs (limited by batch_size, most recent first)
        let email_ids = match kg_extraction::get_unprocessed_emails(
            db.async_connection.clone(),
            credential_id,
            batch_size,
        )
        .await
        {
            Ok(ids) => ids,
            Err(e) => {
                let error_msg = format!(
                    "Failed to get unprocessed emails for credential {}: {}",
                    credential_id, e
                );
                tracing::error!("{}", error_msg);
                state.fail_account(credential_id, error_msg.clone());
                accounts_processed.push(AccountResult {
                    credential_id,
                    identifier: identifier.clone(),
                    emails_processed: 0,
                    total_emails: total_count,
                    unprocessed_remaining: unprocessed_count,
                    success: false,
                    error: Some(error_msg.clone()),
                });
                global_errors.push(error_msg);
                continue;
            }
        };

        let batch_count = email_ids.len();
        tracing::info!(
            "Processing {} unprocessed emails for credential {} (batch of {})",
            batch_count,
            credential_id,
            batch_size
        );

        // Process each email
        let mut emails_processed = 0_usize;
        let mut batch_errors = Vec::new();

        for email_id in email_ids {
            state.update_account_progress(credential_id, email_id, false); // Set as "in progress"

            match process_single_email(
                db.async_connection.clone(),
                email_id,
                llm_client.clone(),
                storage.clone(),
                persistence.clone(),
                search_provider.clone(),
                all_passes,
            )
            .await
            {
                Ok(_) => {
                    state.update_account_progress(credential_id, email_id, true);
                    emails_processed += 1;
                }
                Err(e) => {
                    state.update_account_progress(credential_id, email_id, false);
                    let error_msg = format!("Failed to process email {}: {}", email_id, e);
                    tracing::warn!("{}", error_msg);
                    batch_errors.push(error_msg);
                }
            }
        }

        // Recalculate remaining after processing
        let remaining_after = match kg_extraction::count_unprocessed_emails(
            db.async_connection.clone(),
            credential_id,
        )
        .await
        {
            Ok(c) => c,
            Err(_) => unprocessed_count - emails_processed as i64, // Fallback estimate
        };

        state.complete_account(credential_id);

        accounts_processed.push(AccountResult {
            credential_id,
            identifier: identifier.clone(),
            emails_processed,
            total_emails: total_count,
            unprocessed_remaining: remaining_after,
            success: batch_errors.is_empty() || emails_processed > 0,
            error: if batch_errors.is_empty() {
                None
            } else {
                Some(batch_errors.join("; "))
            },
        });

        total_emails_processed += emails_processed;

        // Add batch errors to global errors (but don't fail the whole operation)
        global_errors.extend(batch_errors);
    }

    let all_success = accounts_processed.iter().all(|a| a.success);

    Ok(HttpResponse::Ok().json(KgExtractionResponse {
        success: all_success && global_errors.is_empty(),
        message: format!(
            "Processed {} emails across {} account(s)",
            total_emails_processed,
            accounts_processed.len()
        ),
        accounts_processed,
        total_emails_processed,
        errors: global_errors,
    }))
}

/// Process a single email through the KG extraction pipeline
async fn process_single_email(
    async_conn: crate::database::AsyncDbConnection,
    email_id: i64,
    llm_client: Arc<OllamaClient>,
    storage: Arc<dyn AgentStorage>,
    persistence: Arc<KgPersistenceLayer>,
    search_provider: Arc<DbEntitySearchProvider>,
    all_passes: bool,
) -> anyhow::Result<()> {
    // Load email
    let email = emails_db::get_email(async_conn, email_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load email: {}", e))?;

    let simple = simple_email_content(
        email.subject.as_deref(),
        email.body_text.as_deref(),
        email.body_html.as_deref(),
    );

    tracing::debug!("Processing email {}: {}", email.id, simple.subject);

    // Step 1: Document labeling (skip if all_passes)
    let label = if all_passes {
        tracing::debug!("Skipping document labeler (all_passes=true)");
        None
    } else {
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
            .map_err(|e| anyhow::anyhow!("Failed to create labeler session: {}", e))?;

        let labeler = TemplateDocumentLabelerAgent::new(
            llm_client.clone(),
            storage.clone(),
            MINISTRAL_3_3B_ID.to_string(),
            simple.body.clone(),
        );

        match labeler.execute(label_session_id).await {
            Ok(label) => {
                tracing::debug!(
                    "Email {} label: doc_type={:?} has_bill={} has_transaction={} has_event={} has_order={}",
                    email.id,
                    label.doc_type,
                    label.has_bill,
                    label.has_transaction,
                    label.has_event,
                    label.has_order,
                );
                Some(label)
            }
            Err(e) => {
                tracing::warn!("Document labeler failed for email {}: {}", email.id, e);
                None
            }
        }
    };

    // Step 2: KG extraction
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
        .map_err(|e| anyhow::anyhow!("Failed to create KG session: {}", e))?;

    let email_content = format!("Subject: {}\n\n{}", simple.subject, simple.body);

    let mut agent = KgEmailExtractionAgent::new(
        llm_client,
        storage,
        persistence,
        MINISTRAL_3_3B_ID.to_string(),
        email_content,
    )
    .with_search_provider(search_provider)
    .with_source_email_id(email.id)
    .with_sender(email.from_name.clone(), Some(email.from_address.clone()));

    if let Some(label) = label {
        agent = agent.with_label(label);
    }

    agent
        .execute(kg_session_id)
        .await
        .map_err(|e| anyhow::anyhow!("KG extraction failed: {}", e))?;

    tracing::debug!("KG extraction complete for email {}", email.id);

    Ok(())
}
