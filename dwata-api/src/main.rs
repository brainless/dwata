use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::prelude::*;

mod config;
mod database;
mod handlers;
mod helpers;
mod integrations;
mod jobs;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Hello World"
    }))
}

#[get("/health")]
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

#[get("/settings")]
async fn get_settings(data: web::Data<handlers::settings::SettingsAppState>) -> impl Responder {
    handlers::settings::get_settings(data).await
}

#[post("/settings/api-keys")]
async fn update_api_keys(
    data: web::Data<handlers::settings::SettingsAppState>,
    request: web::Json<shared_types::UpdateApiKeysRequest>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    handlers::settings::update_api_keys(data, request, req).await
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    log_file_path: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

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
    let settings_state = handlers::settings::SettingsAppState {
        config: config_arc.clone(),
    };

    // Get server config or use defaults
    let (host, port) = if let Some(server_config) = &config.server {
        (server_config.host.clone(), server_config.port)
    } else {
        ("127.0.0.1".to_string(), 8080)
    };

    tracing::info!("Server will listen on {}:{}", host, port);

    // Initialize OAuth components
    let google_oauth_config = config.google_oauth.unwrap_or_default();
    let redirect_uri = format!("http://{}:{}/api/oauth/google/callback", host, port);
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
    tracing::info!("Preloading credentials into keyring cache...");
    match crate::database::credentials::list_credentials(db.async_connection.clone(), false).await {
        Ok(credentials) => {
            let preload_list: Vec<_> = credentials
                .iter()
                .filter(|c| c.credential_type.requires_keychain())
                .map(|c| (c.credential_type.clone(), c.identifier.clone(), c.username.clone()))
                .collect();

            tracing::info!("Found {} credentials to preload", preload_list.len());
            keyring_service.preload_credentials(preload_list).await;
        }
        Err(e) => {
            tracing::warn!("Failed to preload credentials: {}", e);
        }
    }

    // Initialize download manager
    let download_manager = Arc::new(jobs::download_manager::DownloadManager::new(
        db.async_connection.clone(),
        token_cache.clone(),
        oauth_client.clone(),
        keyring_service.clone(),
    ));

    // Initialize extraction manager
    let extraction_manager = Arc::new(jobs::extraction_manager::ExtractionManager::new(
        db.async_connection.clone(),
    ));

    // Initialize financial extraction manager
    let financial_extraction_manager = Arc::new(
        jobs::financial_extraction_manager::FinancialExtractionManager::new(
            db.async_connection.clone(),
        ),
    );

    // Restore interrupted jobs on startup
    if let Err(e) = download_manager.restore_interrupted_jobs().await {
        tracing::warn!("Failed to restore interrupted jobs: {}", e);
    }

    // Ensure all credentials have download jobs (auto-create if missing)
    if let Err(e) = download_manager.ensure_jobs_for_all_credentials().await {
        tracing::warn!("Failed to ensure jobs for all credentials: {}", e);
    }

    // Spawn background task for initial sync (delayed to allow full initialization)
    let manager_clone_startup = download_manager.clone();
    let extraction_manager_clone_startup = financial_extraction_manager.clone();
    tokio::spawn(async move {
        // Wait 2 seconds for server to fully initialize
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        if manager_clone_startup.is_shutting_down() {
            return;
        }

        tracing::info!("Running initial sync after startup delay");

        // Run initial sync to check for new emails
        if let Err(e) = manager_clone_startup.sync_all_jobs().await {
            tracing::warn!("Failed to run initial sync: {}", e);
        }

        // Run initial financial extraction
        match extraction_manager_clone_startup.extract_from_emails(None, None).await {
            Ok(count) => {
                tracing::info!("Financial extraction completed on startup: {} transactions extracted", count);
            }
            Err(e) => {
                tracing::warn!("Failed to run initial financial extraction: {}", e);
            }
        }
    });

    // Spawn historical backfill on startup
    let manager_clone_backfill = download_manager.clone();
    tokio::spawn(async move {
        // Wait 10 seconds to allow recent sync to complete first
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        if manager_clone_backfill.is_shutting_down() {
            return;
        }

        tracing::info!("Starting historical backfill on startup");

        // Get all credentials and start historical backfill for each
        match crate::database::credentials::list_credentials(
            manager_clone_backfill.get_db_connection(),
            false,
        ).await {
            Ok(credentials) => {
                for credential in credentials {
                    if credential.credential_type.requires_keychain() {
                        if let Err(e) = manager_clone_backfill.start_historical_backfill(credential.id).await {
                            tracing::warn!("Failed to start historical backfill for credential {}: {}", credential.id, e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get credentials for historical backfill: {}", e);
            }
        }
    });

    // Spawn periodic sync task (every 5 minutes)
    let manager_clone = download_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            if manager_clone.is_shutting_down() {
                break;
            }
            if let Err(e) = manager_clone.sync_all_jobs().await {
                tracing::error!("Periodic sync failed: {}", e);
            }
        }
    });

    println!("Starting server on {}:{}", host, port);

    let download_manager_for_server = download_manager.clone();
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
                .max_age(3600)
        } else {
            Cors::default()
                .allow_any_origin()
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec!["Authorization", "Accept", "Content-Type"])
                .max_age(3600)
        };

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(settings_state.clone()))
            .app_data(web::Data::new(download_manager_for_server.clone()))
            .app_data(web::Data::new(extraction_manager.clone()))
            .app_data(web::Data::new(financial_extraction_manager.clone()))
            .app_data(web::Data::new(oauth_client.clone()))
            .app_data(web::Data::new(state_manager.clone()))
            .app_data(web::Data::new(token_cache.clone()))
            .app_data(web::Data::new(keyring_service.clone()))
            .service(hello)
            .service(health)
            .service(get_settings)
            .service(update_api_keys)
            .route("/api/credentials", web::post().to(handlers::credentials::create_credential))
            .route("/api/credentials", web::get().to(handlers::credentials::list_credentials))
            .route("/api/credentials/{id}", web::get().to(handlers::credentials::get_credential))
            .route("/api/credentials/{id}/password", web::get().to(handlers::credentials::get_password))
            .route("/api/credentials/{id}", web::put().to(handlers::credentials::update_credential))
            .route("/api/credentials/{id}", web::delete().to(handlers::credentials::delete_credential))
            .route("/api/credentials/gmail/initiate", web::post().to(handlers::oauth::initiate_gmail_oauth))
            .route("/api/oauth/google/callback", web::get().to(handlers::oauth::google_oauth_callback))
            .route("/api/downloads", web::post().to(handlers::downloads::create_download_job))
            .route("/api/downloads", web::get().to(handlers::downloads::list_download_jobs))
            .route("/api/downloads/{id}", web::get().to(handlers::downloads::get_download_job))
            .route("/api/downloads/{id}/start", web::post().to(handlers::downloads::start_download))
            .route("/api/downloads/{id}/pause", web::post().to(handlers::downloads::pause_download))
            .route("/api/downloads/{id}", web::delete().to(handlers::downloads::delete_download_job))
            .route("/api/extractions", web::post().to(handlers::extraction_jobs::create_extraction_job))
            .route("/api/extractions", web::get().to(handlers::extraction_jobs::list_extraction_jobs))
            .route("/api/extractions/{id}", web::get().to(handlers::extraction_jobs::get_extraction_job))
            .route("/api/extractions/{id}/start", web::post().to(handlers::extraction_jobs::start_extraction))
            .route("/api/emails", web::get().to(handlers::emails::list_emails))
            .route("/api/emails/{id}", web::get().to(handlers::emails::get_email))
            .route("/api/emails/{id}/labels", web::get().to(handlers::emails::get_email_labels))
            .route("/api/credentials/{credential_id}/folders", web::get().to(handlers::folders::list_folders))
            .route("/api/folders/{folder_id}", web::get().to(handlers::folders::get_folder))
            .route("/api/folders/{folder_id}/emails", web::get().to(handlers::folders::list_folder_emails))
            .route("/api/credentials/{credential_id}/labels", web::get().to(handlers::labels::list_labels))
            .route("/api/labels/{label_id}", web::get().to(handlers::labels::get_label))
            .route("/api/labels/{label_id}/emails", web::get().to(handlers::labels::list_label_emails))
            .route("/api/events", web::get().to(handlers::events::list_events))
            .route("/api/events/{id}", web::get().to(handlers::events::get_event))
            .route("/api/contacts", web::get().to(handlers::contacts::list_contacts))
            .route("/api/contacts/{id}", web::get().to(handlers::contacts::get_contact))
            .route("/api/contacts/{id}/links", web::get().to(handlers::contacts::get_contact_links))
            .route("/api/companies", web::get().to(handlers::companies::list_companies))
            .route("/api/companies/{id}", web::get().to(handlers::companies::get_company))
            .route("/api/positions", web::get().to(handlers::positions::list_positions))
            .route("/api/positions/{id}", web::get().to(handlers::positions::get_position))
            .route("/api/contacts/{id}/positions", web::get().to(handlers::positions::list_contact_positions))
            .route("/api/financial/transactions", web::get().to(handlers::financial::list_transactions))
            .route("/api/financial/summary", web::get().to(handlers::financial::get_summary))
            .route("/api/financial/extract", web::post().to(handlers::financial::trigger_extraction))
            .route("/api/financial/extractions/summary", web::get().to(handlers::financial::get_extraction_summary))
            .route("/api/financial/extractions/attempts", web::get().to(handlers::financial::list_extraction_attempts))
            .route("/api/financial/patterns", web::get().to(handlers::financial::list_patterns))
            .route("/api/financial/patterns", web::post().to(handlers::financial::create_pattern))
            .route("/api/financial/patterns/{id}", web::get().to(handlers::financial::get_pattern))
            .route("/api/financial/patterns/{id}", web::put().to(handlers::financial::update_pattern))
            .route("/api/financial/patterns/{id}/toggle", web::patch().to(handlers::financial::toggle_pattern))
            .service(handlers::pattern_generation::generate_pattern)
    })
    .bind((host.as_str(), port))?
    .run();

    let handle = server.handle();
    let shutdown_manager = download_manager.clone();

    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to listen for Ctrl+C: {}", e);
            return;
        }

        tracing::info!("Ctrl+C received, shutting down...");
        if let Err(e) = shutdown_manager.shutdown().await {
            tracing::warn!("Failed to shutdown download manager cleanly: {}", e);
        }

        handle.stop(true).await;
    });

    server.await
}
