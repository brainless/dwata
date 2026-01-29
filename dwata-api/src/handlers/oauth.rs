use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use oauth2::TokenResponse;

use crate::helpers::google_oauth::GoogleOAuthClient;
use crate::helpers::oauth_state::OAuthStateManager;
use crate::helpers::keyring_service::KeyringService;
use crate::database::credentials as db;
use shared_types::credential::{CredentialType, CreateCredentialRequest};

#[derive(Serialize)]
pub struct InitiateOAuthResponse {
    pub authorization_url: String,
}

pub async fn initiate_gmail_oauth(
    oauth_client: web::Data<Arc<GoogleOAuthClient>>,
    state_manager: web::Data<Arc<OAuthStateManager>>,
) -> Result<HttpResponse> {
    let (auth_url, csrf_token, pkce_verifier) = oauth_client.authorize_url();

    state_manager.store_verifier(csrf_token.secret().to_string(), pkce_verifier).await;

    Ok(HttpResponse::Ok().json(InitiateOAuthResponse {
        authorization_url: auth_url,
    }))
}

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    code: String,
    state: String,
    error: Option<String>,
}

pub async fn google_oauth_callback(
    query: web::Query<OAuthCallbackQuery>,
    oauth_client: web::Data<Arc<GoogleOAuthClient>>,
    state_manager: web::Data<Arc<OAuthStateManager>>,
    db: web::Data<Arc<crate::database::Database>>,
    token_cache: web::Data<Arc<crate::helpers::token_cache::TokenCache>>,
) -> Result<HttpResponse> {
    if let Some(error) = &query.error {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("OAuth authorization failed: {}", error)
        })));
    }

    let pkce_verifier = state_manager
        .retrieve_verifier(&query.state)
        .await
        .ok_or_else(|| {
            actix_web::error::ErrorBadRequest("Invalid state parameter")
        })?;

    let token_response = oauth_client
        .exchange_code(query.code.clone(), pkce_verifier)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Token exchange failed: {}", e)))?;

    let access_token = token_response.access_token().secret();
    let refresh_token = token_response
        .refresh_token()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("No refresh token received"))?
        .secret();
    let expires_in = token_response
        .expires_in()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("No expiry information"))?
        .as_secs() as i64;

    let email = get_user_email(access_token).await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to get user email: {}", e)))?;

    let credential_request = CreateCredentialRequest {
        credential_type: CredentialType::OAuth,
        identifier: format!("gmail_{}", email.replace('@', "_at_")),
        username: email.clone(),
        password: refresh_token.clone(),
        service_name: Some("imap.gmail.com".to_string()),
        port: Some(993),
        use_tls: Some(true),
        notes: Some("Gmail OAuth2 credential".to_string()),
        extra_metadata: Some(serde_json::json!({
            "provider": "google",
            "scopes": ["https://mail.google.com/"]
        }).to_string()),
    };

    let metadata = db::insert_credential(db.async_connection.clone(), &credential_request)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to store credential: {}", e)))?;

    KeyringService::set_password(
        &CredentialType::OAuth,
        &credential_request.identifier,
        &email,
        refresh_token,
    )
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to store refresh token: {}", e)))?;

    token_cache.store_token(metadata.id, access_token.to_string(), expires_in).await;

    Ok(HttpResponse::Ok().body(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head><title>Gmail Connected</title></head>
        <body>
            <h1>Gmail Account Connected Successfully!</h1>
            <p>Email: {}</p>
            <p>You can close this window and return to dwata.</p>
            <script>
                setTimeout(() => window.close(), 2000);
            </script>
        </body>
        </html>
        "#,
        email
    )))
}

async fn get_user_email(access_token: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await?;

    let user_info: serde_json::Value = response.json().await?;
    let email = user_info["email"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No email in response"))?
        .to_string();

    Ok(email)
}
