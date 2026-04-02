use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

use crate::contact_extractor;
use crate::database::Database;

pub async fn run_extraction(db: web::Data<Arc<Database>>) -> impl Responder {
    let conn = db.async_connection.clone();

    match contact_extractor::extract_contacts_from_emails(conn).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(err) => {
            tracing::error!(error = %err, "Contact extraction failed");
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": err.to_string()
            }))
        }
    }
}
