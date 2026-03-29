use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::SubscriptionsResponse;
use std::sync::Arc;

use crate::database::subscriptions as db;
use crate::database::Database;

pub async fn list_subscriptions(database: web::Data<Arc<Database>>) -> ActixResult<HttpResponse> {
    let subscriptions = db::list_subscriptions(database.async_connection.clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(SubscriptionsResponse { subscriptions }))
}

pub async fn get_subscription(
    database: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let subscription_id = path.into_inner();

    let subscription = db::get_subscription(database.async_connection.clone(), subscription_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(subscription))
}
