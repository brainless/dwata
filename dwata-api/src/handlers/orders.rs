use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::OrdersResponse;
use std::sync::Arc;

use crate::database::orders as db;
use crate::database::Database;

pub async fn list_orders(database: web::Data<Arc<Database>>) -> ActixResult<HttpResponse> {
    let orders = db::list_orders(database.async_connection.clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(OrdersResponse { orders }))
}

pub async fn get_order(
    database: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let order_id = path.into_inner();

    let order = db::get_order(database.async_connection.clone(), order_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(order))
}
