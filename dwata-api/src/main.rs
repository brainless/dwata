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

    // Initialize download manager
    let download_manager = Arc::new(jobs::download_manager::DownloadManager::new(db.async_connection.clone()));

    // Restore interrupted jobs on startup
    let _ = download_manager.restore_interrupted_jobs().await;

    // Spawn periodic sync task (every 5 minutes)
    let manager_clone = download_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            if let Err(e) = manager_clone.sync_all_jobs().await {
                tracing::error!("Periodic sync failed: {}", e);
            }
        }
    });

    // Get server config or use defaults
    let (host, port) = if let Some(server_config) = &config.server {
        (server_config.host.clone(), server_config.port)
    } else {
        ("127.0.0.1".to_string(), 8080)
    };

    println!("Starting server on {}:{}", host, port);

    HttpServer::new(move || {
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
            .app_data(web::Data::new(download_manager.clone()))
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
            .route("/api/downloads", web::post().to(handlers::downloads::create_download_job))
            .route("/api/downloads", web::get().to(handlers::downloads::list_download_jobs))
            .route("/api/downloads/{id}", web::get().to(handlers::downloads::get_download_job))
            .route("/api/downloads/{id}/start", web::post().to(handlers::downloads::start_download))
            .route("/api/downloads/{id}/pause", web::post().to(handlers::downloads::pause_download))
            .route("/api/downloads/{id}", web::delete().to(handlers::downloads::delete_download_job))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
