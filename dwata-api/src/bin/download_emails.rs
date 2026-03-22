//! CLI tool to download emails for a specific account.
//!
//! Usage:
//!   cargo run --bin download_emails -- --email user@example.com --direction recent
//!   cargo run --bin download_emails -- --email user@example.com --direction backfill
//!
//! The tool reuses the same EmailSyncManager code path as the HTTP server,
//! making it easy to test email downloading independently.

use anyhow::{Context, Result};
use clap::Parser;
use dwata_api::database::credentials::list_credentials;
use dwata_api::helpers::database::initialize_database;
use dwata_api::helpers::google_oauth::GoogleOAuthClient;
use dwata_api::helpers::keyring_service::KeyringService;
use dwata_api::helpers::token_cache::TokenCache;
use dwata_api::jobs::email_sync_manager::EmailSyncManager;
use shared_types::download::EmailSyncDirection;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "download_emails",
    about = "Download emails for a specific account (recent or backfill)"
)]
struct Args {
    /// Email address of the account to sync (must match a stored credential)
    #[arg(long)]
    email: String,

    /// Sync direction: 'recent' (new emails) or 'backfill' (older emails)
    #[arg(long, default_value = "recent")]
    direction: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("dwata_api=info".parse()?)
                .add_directive("info".parse()?),
        )
        .with_target(false)
        .init();

    let args = Args::parse();

    let direction = match args.direction.to_lowercase().as_str() {
        "recent" => EmailSyncDirection::Recent,
        "backfill" => EmailSyncDirection::Backfill,
        other => anyhow::bail!("Unknown direction '{}'. Use 'recent' or 'backfill'.", other),
    };

    tracing::info!(email = %args.email, direction = ?direction, "starting email download");

    // Initialize database
    let db = initialize_database().context("Failed to initialize database")?;
    tracing::info!("Database initialized");

    // Load config (needed for OAuth)
    let config = dwata_api::config::ApiConfig::load()
        .map(|(c, _)| c)
        .unwrap_or_default();

    // OAuth + keyring setup
    let mut google_oauth_config = config.google_oauth.clone().unwrap_or_default();
    google_oauth_config.apply_compiled_defaults();
    let oauth_client = Arc::new(
        GoogleOAuthClient::new(
            &google_oauth_config.client_id,
            google_oauth_config.client_secret.as_deref(),
            "http://localhost:8080/api/oauth/google/callback",
        )
        .context("Failed to initialize OAuth client")?,
    );

    let token_cache = Arc::new(TokenCache::new());
    let keyring_service = Arc::new(KeyringService::new());

    // Load credentials from keychain
    if KeyringService::has_master_credentials() {
        match keyring_service.get_master_credentials().await {
            Ok(creds) => tracing::info!("Loaded {} credentials from keychain", creds.len()),
            Err(e) => tracing::warn!("Could not load keychain credentials: {}", e),
        }
    }

    // Find the credential matching the given email address
    let all_credentials = list_credentials(db.async_connection.clone(), false)
        .await
        .context("Failed to list credentials")?;

    let credential = all_credentials
        .into_iter()
        .find(|c| c.username.eq_ignore_ascii_case(&args.email))
        .with_context(|| {
            format!(
                "No credential found for email '{}'. \
                 Make sure the account is configured in dwata.",
                args.email
            )
        })?;

    tracing::info!(
        credential_id = credential.id,
        email = %credential.username,
        identifier = %credential.identifier,
        credential_type = ?credential.credential_type,
        "found credential"
    );

    // Build a minimal EmailSyncManager
    let credential_semaphores: Arc<Mutex<HashMap<i64, Arc<Semaphore>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    // Run the sync synchronously (blocking until complete)
    EmailSyncManager::run_imap_sync(
        db.async_connection.clone(),
        credential.id,
        &direction,
        token_cache,
        oauth_client,
        keyring_service,
        credential_semaphores,
        shutdown_flag,
    )
    .await
    .with_context(|| format!("Email sync failed for {} ({:?})", args.email, direction))?;

    tracing::info!(email = %args.email, "email download completed successfully");
    Ok(())
}
