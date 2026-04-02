use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::OrganisationsWithCountsResponse;
use std::sync::Arc;

use crate::database::organisations as db;
use crate::database::Database;

pub async fn list_organisations(database: web::Data<Arc<Database>>) -> ActixResult<HttpResponse> {
    let organisations = db::list_organisations_with_counts(database.async_connection.clone(), 500)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(OrganisationsWithCountsResponse { organisations }))
}

pub async fn get_organisation(
    database: web::Data<Arc<Database>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let organisation_id = path.into_inner();

    let organisation = db::get_organisation(database.async_connection.clone(), organisation_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(organisation))
}
