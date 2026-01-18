use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use std::sync::Arc;

mod database;
mod helpers;

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize database
    let db = helpers::database::initialize_database().expect("Failed to initialize database");

    println!(
        "Database initialized at: {:?}",
        helpers::database::get_db_path().unwrap()
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .service(hello)
            .service(health)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
