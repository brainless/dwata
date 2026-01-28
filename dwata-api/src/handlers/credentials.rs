use actix_web::{web, HttpResponse, Result};
use serde::Deserialize;
use shared_types::credential::{
    CreateCredentialRequest, CredentialListResponse, PasswordResponse,
    UpdateCredentialRequest,
};
use std::sync::Arc;
use actix_web::http::header::HeaderName;

use crate::database::credentials as db;
use crate::helpers::keyring_service::{KeyringError, KeyringService};

#[derive(Debug)]
enum CredentialError {
    Validation(String),
    NotFound,
    Duplicate,
    KeychainUnavailable(String),
    InconsistentState(String),
    Internal(String),
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialError::Validation(msg) => write!(f, "{}", msg),
            CredentialError::NotFound => write!(f, "Credential not found"),
            CredentialError::Duplicate => write!(f, "A credential with this identifier already exists"),
            CredentialError::KeychainUnavailable(msg) => write!(f, "{}", msg),
            CredentialError::InconsistentState(msg) => write!(f, "{}", msg),
            CredentialError::Internal(msg) => write!(f, "{}", msg),
        }
    }
}

impl actix_web::error::ResponseError for CredentialError {
    fn error_response(&self) -> HttpResponse {
        match self {
            CredentialError::Validation(msg) => {
                HttpResponse::BadRequest().json(serde_json::json!({ "error": msg }))
            }
            CredentialError::NotFound => {
                HttpResponse::NotFound().json(serde_json::json!({ "error": "Credential not found" }))
            }
            CredentialError::Duplicate => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "A credential with this identifier already exists"
                }))
            }
            CredentialError::KeychainUnavailable(msg) => {
                HttpResponse::ServiceUnavailable().json(serde_json::json!({ "error": msg }))
            }
            CredentialError::InconsistentState(msg) => {
                HttpResponse::InternalServerError().json(serde_json::json!({ "error": msg }))
            }
            CredentialError::Internal(msg) => {
                HttpResponse::InternalServerError().json(serde_json::json!({ "error": msg }))
            }
        }
    }
}

fn add_security_header(mut response: HttpResponse) -> HttpResponse {
    response.headers_mut().insert(
        HeaderName::from_static("x-security-warning"),
        "This API has no authentication. Enable only in trusted environments."
            .parse()
            .unwrap(),
    );
    response
}

pub async fn create_credential(
    db: web::Data<Arc<crate::database::Database>>,
    request: web::Json<CreateCredentialRequest>,
) -> Result<HttpResponse> {
    let req = request.into_inner();

    if req.identifier.trim().is_empty() {
        return Err(CredentialError::Validation(
            "Identifier cannot be empty".to_string(),
        )
        .into());
    }
    if req.username.trim().is_empty() {
        return Err(CredentialError::Validation(
            "Username cannot be empty".to_string(),
        )
        .into());
    }
    if req.password.trim().is_empty() {
        return Err(CredentialError::Validation(
            "Password cannot be empty".to_string(),
        )
        .into());
    }

    KeyringService::set_password(
        &req.credential_type,
        &req.identifier,
        &req.username,
        &req.password,
    )
    .map_err(|e| match e {
        KeyringError::ServiceUnavailable(msg) => CredentialError::KeychainUnavailable(msg),
        _ => CredentialError::Internal(format!("Failed to store password: {}", e)),
    })?;

    let metadata = db::insert_credential(db.async_connection.clone(), &req)
        .await
        .map_err(|e| {
            let _ = KeyringService::delete_password(
                &req.credential_type,
                &req.identifier,
                &req.username,
            );

            match e {
                db::CredentialDbError::DuplicateIdentifier => CredentialError::Duplicate,
                db::CredentialDbError::DatabaseError(msg) => CredentialError::Internal(msg),
                _ => CredentialError::Internal(e.to_string()),
            }
        })?;

    Ok(add_security_header(HttpResponse::Created().json(metadata)))
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    include_inactive: bool,
}

pub async fn list_credentials(
    db: web::Data<Arc<crate::database::Database>>,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse> {
    let credentials = db::list_credentials(db.async_connection.clone(), query.include_inactive)
        .await
        .map_err(|e| CredentialError::Internal(e.to_string()))?;

    Ok(add_security_header(
        HttpResponse::Ok().json(CredentialListResponse { credentials }),
    ))
}

pub async fn get_credential(
    db: web::Data<Arc<crate::database::Database>>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let credential = db::get_credential(db.async_connection.clone(), id)
        .await
        .map_err(|e| match e {
            db::CredentialDbError::NotFound => CredentialError::NotFound,
            _ => CredentialError::Internal(e.to_string()),
        })?;

    Ok(add_security_header(HttpResponse::Ok().json(credential)))
}

pub async fn get_password(
    db: web::Data<Arc<crate::database::Database>>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let credential = db::get_credential(db.async_connection.clone(), id)
        .await
        .map_err(|e| match e {
            db::CredentialDbError::NotFound => CredentialError::NotFound,
            _ => CredentialError::Internal(e.to_string()),
        })?;

    let password = KeyringService::get_password(
        &credential.credential_type,
        &credential.identifier,
        &credential.username,
    )
    .map_err(|e| match e {
        KeyringError::NotFound => CredentialError::InconsistentState(
            "Credential exists in database but not in keychain".to_string(),
        ),
        KeyringError::ServiceUnavailable(msg) => CredentialError::KeychainUnavailable(msg),
        KeyringError::OperationFailed(msg) => CredentialError::Internal(msg),
    })?;

    let _ = db::update_last_accessed(db.async_connection.clone(), id).await;

    Ok(add_security_header(
        HttpResponse::Ok().json(PasswordResponse { password }),
    ))
}

pub async fn update_credential(
    db: web::Data<Arc<crate::database::Database>>,
    path: web::Path<i64>,
    request: web::Json<UpdateCredentialRequest>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let req = request.into_inner();

    let existing = db::get_credential(db.async_connection.clone(), id)
        .await
        .map_err(|e| match e {
            db::CredentialDbError::NotFound => CredentialError::NotFound,
            _ => CredentialError::Internal(e.to_string()),
        })?;

    if let Some(ref new_password) = req.password {
        KeyringService::update_password(
            &existing.credential_type,
            &existing.identifier,
            &existing.username,
            new_password,
        )
        .map_err(|e| match e {
            KeyringError::ServiceUnavailable(msg) => CredentialError::KeychainUnavailable(msg),
            _ => CredentialError::Internal(format!("Failed to update password: {}", e)),
        })?;
    }

    let updated = db::update_credential(
        db.async_connection.clone(),
        id,
        req.username,
        req.service_name,
        req.port,
        req.use_tls,
        req.notes,
        req.extra_metadata,
    )
    .await
    .map_err(|e| CredentialError::Internal(e.to_string()))?;

    Ok(add_security_header(HttpResponse::Ok().json(updated)))
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    #[serde(default)]
    hard: bool,
}

pub async fn delete_credential(
    db: web::Data<Arc<crate::database::Database>>,
    path: web::Path<i64>,
    query: web::Query<DeleteQuery>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let credential = db::get_credential(db.async_connection.clone(), id)
        .await
        .map_err(|e| match e {
            db::CredentialDbError::NotFound => CredentialError::NotFound,
            _ => CredentialError::Internal(e.to_string()),
        })?;

    if query.hard {
        let _ = KeyringService::delete_password(
            &credential.credential_type,
            &credential.identifier,
            &credential.username,
        )
        .map_err(|e| match e {
            KeyringError::NotFound => {
                CredentialError::InconsistentState(
                    "Keychain entry not found, deleting database record".to_string(),
                )
            }
            KeyringError::ServiceUnavailable(msg) => CredentialError::KeychainUnavailable(msg),
            KeyringError::OperationFailed(msg) => CredentialError::Internal(msg),
        })
        .ok();

        db::hard_delete_credential(db.async_connection.clone(), id)
            .await
            .map_err(|e| CredentialError::Internal(e.to_string()))?;
    } else {
        db::soft_delete_credential(db.async_connection.clone(), id)
            .await
            .map_err(|e| match e {
                db::CredentialDbError::NotFound => CredentialError::NotFound,
                _ => CredentialError::Internal(e.to_string()),
            })?;
    }

    Ok(add_security_header(HttpResponse::NoContent().finish()))
}
