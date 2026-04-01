use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::prelude::*;

mod config;
mod database;
mod handlers;
mod helpers;
mod integrations;
mod jobs;
mod search;
mod state;

const GUI_EMBED_ENABLED: bool = false;

mod gui_embed {
    use super::*;

    pub async fn serve_gui(_req: HttpRequest) -> HttpResponse {
        HttpResponse::NotFound().finish()
    }
}

#[get("/api/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Hello World"
    }))
}

#[get("/api/health")]
async fn health(db: web::Data<Arc<database::Database>>) -> impl Responder {
    // Test database connection
    match db.connection.lock() {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "status": "healthy",
            "database": "connected"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "unhealthy",
            "database": "disconnected"
        })),
    }
}

#[get("/api/settings")]
async fn get_settings(data: web::Data<handlers::settings::SettingsAppState>) -> impl Responder {
    handlers::settings::get_settings(data).await
}

#[post("/api/settings/ai-provider-api-keys")]
async fn update_ai_provider_api_keys(
    data: web::Data<handlers::settings::SettingsAppState>,
    request: web::Json<shared_types::UpdateAiProviderApiKeysRequest>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    handlers::settings::update_ai_provider_api_keys(data, request, req).await
}

#[post("/api/settings/oauth-client-apps")]
async fn update_oauth_client_apps(
    data: web::Data<handlers::settings::SettingsAppState>,
    request: web::Json<shared_types::UpdateOAuthClientAppsRequest>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    handlers::settings::update_oauth_client_apps(data, request, req).await
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    log_file_path: Option<String>,
    #[arg(long)]
    no_open: bool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tantivy=warn"));

    if let Some(log_path) = args.log_file_path {
        let log_path = std::path::Path::new(&log_path);
        let file_appender = tracing_appender::rolling::never(
            log_path.parent().unwrap_or(std::path::Path::new(".")),
            log_path
                .file_name()
                .unwrap_or(std::ffi::OsStr::new("dwata-api.log")),
        );
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        std::mem::forget(guard);

        tracing_subscriber::registry()
            .with(env_filter.clone())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(true)
                    .with_writer(std::io::stdout),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(non_blocking),
            )
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

    // Initialize database
    let db = helpers::database::initialize_database().expect("Failed to initialize database");

    println!(
        "Database initialized at: {:?}",
        helpers::database::get_db_path().unwrap()
    );

    // Load config
    let (config, _) = config::ApiConfig::load().expect("Failed to load config");
    let config_arc = Arc::new(std::sync::RwLock::new(config.clone()));
    let config = Arc::new(config);
    let settings_state = handlers::settings::SettingsAppState {
        config: config_arc.clone(),
    };

    let search_index_path = config
        .search
        .as_ref()
        .and_then(|s| s.index_path.as_ref())
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::data_local_dir().map(|d| d.join("dwata").join("tantivy-index")))
        .expect("Failed to resolve search index path");
    let search_index = Arc::new(
        search::tantivy::open_or_create_index(&search_index_path)
            .expect("Failed to initialize tantivy index"),
    );
    tracing::info!("Tantivy index path: {}", search_index_path.display());

    // Start email search index backfill
    let search_index_backfill = search_index.clone();
    let db_for_backfill = db.clone();
    tokio::spawn(async move {
        let mut after_id = 0_i64;
        let page_size = 500_usize;
        let mut total_seen = 0usize;
        let mut total_indexed = 0usize;
        let mut total_failed = 0usize;
        let mut pages_processed = 0usize;
        loop {
            let page = match crate::database::emails::list_emails_for_indexing_page(
                db_for_backfill.async_connection.clone(),
                after_id,
                page_size,
            )
            .await
            {
                Ok(rows) => rows,
                Err(err) => {
                    tracing::warn!("Failed to backfill search index page: {}", err);
                    break;
                }
            };

            if page.is_empty() {
                break;
            }

            total_seen += page.len();
            let page_rows: Vec<(i64, crate::search::tantivy::IndexedTextFields)> = page
                .into_iter()
                .map(|(email, indexed_text)| (email.id, indexed_text))
                .collect();
            let page_count = page_rows.len();
            if let Err(err) = search_index_backfill.index_emails(&page_rows) {
                total_failed += page_count;
                let first_id = page_rows.first().map(|(id, _)| *id);
                let last_id = page_rows.last().map(|(id, _)| *id);
                tracing::warn!(
                    page_size = page_count,
                    first_email_id = first_id,
                    last_email_id = last_id,
                    error = %err,
                    "Backfill page failed to index"
                );
            } else {
                total_indexed += page_count;
            }
            pages_processed += 1;
            if pages_processed % 2 == 0 {
                tracing::info!(
                    pages_processed,
                    total_seen,
                    total_indexed,
                    total_failed,
                    "Search index backfill progress"
                );
            }

            if let Some((last_id, _)) = page_rows.last() {
                after_id = *last_id;
            }
        }
        tracing::info!(
            total_seen,
            total_indexed,
            total_failed,
            "Search index backfill completed"
        );
    });

    // Get server config or use defaults
    let (host, port) = if let Some(server_config) = &config.server {
        (server_config.host.clone(), server_config.port)
    } else {
        ("127.0.0.1".to_string(), 8080)
    };

    tracing::info!("Server will listen on {}:{}", host, port);

    // Initialize OAuth components
    let mut google_oauth_config = config.google_oauth.clone().unwrap_or_default();
    google_oauth_config.apply_compiled_defaults();
    let redirect_uri = format!("http://{}:{}/api/oauth/google/callback", host, port);
    if host != "localhost" {
        tracing::warn!(
            "OAuth redirect URI uses host '{}'. For Google Desktop OAuth, set server.host to 'localhost' to avoid token exchange errors. Redirect URI: {}",
            host,
            redirect_uri
        );
    }
    let oauth_client = Arc::new(
        crate::helpers::google_oauth::GoogleOAuthClient::new(
            &google_oauth_config.client_id,
            google_oauth_config.client_secret.as_deref(),
            &redirect_uri,
        )
        .expect("Failed to initialize OAuth client"),
    );
    let state_manager = Arc::new(crate::helpers::oauth_state::OAuthStateManager::new());
    let token_cache = Arc::new(crate::helpers::token_cache::TokenCache::new());

    // Initialize keyring service with caching
    tracing::info!("Initializing keyring service with 1 hour cache TTL");
    let keyring_service = Arc::new(crate::helpers::keyring_service::KeyringService::new());

    // Preload credentials into cache at startup
    // Always uses master credentials mode (single keychain entry = 1 prompt)
    tracing::info!("Loading credentials from master keychain entry...");

    if crate::helpers::keyring_service::KeyringService::has_master_credentials() {
        match keyring_service.get_master_credentials().await {
            Ok(creds) => {
                tracing::info!(
                    "✅ Loaded {} credentials from master entry (1 keychain prompt)",
                    creds.len()
                );
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️  Master credentials not found: {}. Run migration script if you have existing credentials.",
                    e
                );
            }
        }
    } else {
        tracing::info!("ℹ️  No master credentials found. This is normal for first-time setup.");
        tracing::info!("   New credentials will be automatically stored in master mode.");
    }

    // Initialize email sync manager
    let email_sync_manager = Arc::new(jobs::email_sync_manager::EmailSyncManager::new(
        db.async_connection.clone(),
        token_cache.clone(),
        oauth_client.clone(),
        keyring_service.clone(),
    ));

    // Initialize KG extraction state manager (in-memory only)
    let kg_extraction_state = Arc::new(state::kg_extraction::KgExtractionState::new());

    let email_downloads_auto_start = config
        .email_downloads
        .as_ref()
        .map(|c| c.auto_start)
        .unwrap_or(false);

    if email_downloads_auto_start {
        // Recent sync — starts 2 seconds after server is up, then every 5 minutes
        let manager = email_sync_manager.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if manager.is_shutting_down() {
                return;
            }
            tracing::info!("Running initial recent sync");
            if let Err(e) = manager.sync_all_recent().await {
                tracing::warn!("Initial recent sync failed: {}", e);
            }

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                if manager.is_shutting_down() {
                    break;
                }
                if let Err(e) = manager.sync_all_recent().await {
                    tracing::error!("Periodic recent sync failed: {}", e);
                }
            }
        });

        // Backfill — starts 10 seconds after server is up, then every 10 minutes
        let manager = email_sync_manager.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            if manager.is_shutting_down() {
                return;
            }
            tracing::info!("Running initial backfill");
            if let Err(e) = manager.sync_all_backfill().await {
                tracing::warn!("Initial backfill failed: {}", e);
            }

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                interval.tick().await;
                if manager.is_shutting_down() {
                    break;
                }
                if let Err(e) = manager.sync_all_backfill().await {
                    tracing::error!("Periodic backfill failed: {}", e);
                }
            }
        });
    } else {
        tracing::info!("Email download auto-start disabled (email_downloads.auto_start = false)");
    }

    println!("Starting server on {}:{}", host, port);

    let email_sync_manager_for_server = email_sync_manager.clone();
    let kg_extraction_state_for_server = kg_extraction_state.clone();
    let server = HttpServer::new(move || {
        // Configure CORS
        let cors = if let Some(cors_config) = &config.cors {
            let mut cors_builder = Cors::default();
            for origin in &cors_config.allowed_origins {
                cors_builder = cors_builder.allowed_origin(origin);
            }
            cors_builder
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec!["Authorization", "Accept", "Content-Type"])
                .expose_headers(vec!["x-detect-state-version"])
                .max_age(3600)
        } else {
            Cors::default()
                .allow_any_origin()
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec!["Authorization", "Accept", "Content-Type"])
                .expose_headers(vec!["x-detect-state-version"])
                .max_age(3600)
        };

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(settings_state.clone()))
            .app_data(web::Data::new(email_sync_manager_for_server.clone()))
            .app_data(web::Data::new(oauth_client.clone()))
            .app_data(web::Data::new(state_manager.clone()))
            .app_data(web::Data::new(token_cache.clone()))
            .app_data(web::Data::new(keyring_service.clone()))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(search_index.clone()))
            .app_data(web::Data::new(kg_extraction_state_for_server.clone()))
            .service(hello)
            .service(health)
            .service(get_settings)
            .service(update_ai_provider_api_keys)
            .service(update_oauth_client_apps)
            .route(
                "/api/ollama/status",
                web::get().to(handlers::ollama::ollama_status),
            )
            .route(
                "/api/ollama/models",
                web::get().to(handlers::ollama::ollama_list_models),
            )
            .route(
                "/api/ollama/pull",
                web::post().to(handlers::ollama::ollama_pull_model),
            )
            .route(
                "/api/clear-extracted-data",
                web::post().to(handlers::clear_data::clear_extracted_data),
            )
            .route(
                "/api/kg-extraction/run",
                web::post().to(handlers::kg_extraction::run_kg_extraction),
            )
            .route(
                "/api/kg-extraction/progress",
                web::get().to(handlers::kg_extraction::get_kg_extraction_progress),
            )
            .route(
                "/api/kg-extraction/step-state",
                web::get().to(handlers::kg_extraction::get_extraction_step_state),
            )
            .route(
                "/api/credentials",
                web::post().to(handlers::credentials::create_credential),
            )
            .route(
                "/api/credentials",
                web::get().to(handlers::credentials::list_credentials),
            )
            .route(
                "/api/credentials/{id}",
                web::get().to(handlers::credentials::get_credential),
            )
            .route(
                "/api/credentials/{id}/password",
                web::get().to(handlers::credentials::get_password),
            )
            .route(
                "/api/credentials/{id}",
                web::put().to(handlers::credentials::update_credential),
            )
            .route(
                "/api/credentials/{id}",
                web::delete().to(handlers::credentials::delete_credential),
            )
            .route(
                "/api/credentials/gmail/initiate",
                web::post().to(handlers::oauth::initiate_gmail_oauth),
            )
            .route(
                "/api/oauth/google/callback",
                web::get().to(handlers::oauth::google_oauth_callback),
            )
            .route(
                "/api/email-downloads/sync",
                web::post().to(handlers::email_downloads::trigger_sync),
            )
            .route(
                "/api/email-downloads/sync-all",
                web::post().to(handlers::email_downloads::trigger_sync_all),
            )
            .route("/api/emails", web::get().to(handlers::emails::list_emails))
            .route(
                "/api/emails/by-ids",
                web::post().to(handlers::emails::get_emails_by_ids),
            )
            .route(
                "/api/emails/{id}",
                web::get().to(handlers::emails::get_email),
            )
            .route(
                "/api/emails/{id}/labels",
                web::get().to(handlers::emails::get_email_labels),
            )
            .route("/api/search", web::get().to(handlers::search::search))
            .route(
                "/api/credentials/{credential_id}/folders",
                web::get().to(handlers::folders::list_folders),
            )
            .route(
                "/api/folders/{folder_id}",
                web::get().to(handlers::folders::get_folder),
            )
            .route(
                "/api/folders/{folder_id}/emails",
                web::get().to(handlers::folders::list_folder_emails),
            )
            .route(
                "/api/credentials/{credential_id}/labels",
                web::get().to(handlers::labels::list_labels),
            )
            .route(
                "/api/labels/{label_id}",
                web::get().to(handlers::labels::get_label),
            )
            .route(
                "/api/labels/{label_id}/emails",
                web::get().to(handlers::labels::list_label_emails),
            )
            .route("/api/events", web::get().to(handlers::events::list_events))
            .route(
                "/api/events/{id}",
                web::get().to(handlers::events::get_event),
            )
            .route(
                "/api/locations",
                web::get().to(handlers::locations::list_locations),
            )
            .route(
                "/api/locations/{id}",
                web::get().to(handlers::locations::get_location),
            )
            .route(
                "/api/subscriptions",
                web::get().to(handlers::subscriptions::list_subscriptions),
            )
            .route(
                "/api/subscriptions/{id}",
                web::get().to(handlers::subscriptions::get_subscription),
            )
            .route("/api/orders", web::get().to(handlers::orders::list_orders))
            .route(
                "/api/orders/{id}",
                web::get().to(handlers::orders::get_order),
            )
            .route(
                "/api/persons",
                web::get().to(handlers::persons::list_persons),
            )
            .route(
                "/api/persons/{id}",
                web::get().to(handlers::persons::get_person),
            )
            .route(
                "/api/persons/{id}/links",
                web::get().to(handlers::persons::get_person_links),
            )
            .route(
                "/api/organisations",
                web::get().to(handlers::organisations::list_organisations),
            )
            .route(
                "/api/organisations/{id}",
                web::get().to(handlers::organisations::get_organisation),
            )
            .route(
                "/api/financial/transactions",
                web::get().to(handlers::financial::list_transactions),
            )
            .route(
                "/api/financial/transactions/{id}",
                web::get().to(handlers::financial::get_transaction),
            )
            .route(
                "/api/financial/bills",
                web::get().to(handlers::financial::list_bills),
            )
            .route(
                "/api/financial/bills/{id}",
                web::get().to(handlers::financial::get_bill),
            )
            .default_service(web::route().to(gui_embed::serve_gui))
    })
    .bind((host.as_str(), port))?
    .run();

    let handle = server.handle();
    let shutdown_manager = email_sync_manager.clone();

    let open_in_browser = GUI_EMBED_ENABLED
        && !cfg!(debug_assertions)
        && !args.no_open
        && std::env::var("DWATA_NO_OPEN").is_err();
    if open_in_browser {
        let url = format!("http://{}:{}/", host, port);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if let Err(err) = webbrowser::open(&url) {
                tracing::warn!("Failed to open browser: {}", err);
            }
        });
    }

    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to listen for Ctrl+C: {}", e);
            return;
        }

        tracing::info!("Ctrl+C received, shutting down...");
        if let Err(e) = shutdown_manager.shutdown().await {
            tracing::warn!("Failed to shutdown email sync manager cleanly: {}", e);
        }

        handle.stop(true).await;
    });

    server.await
}
