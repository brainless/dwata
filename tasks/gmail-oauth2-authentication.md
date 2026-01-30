# Task: Gmail OAuth2 Authentication for IMAP Access

## Objective

Implement OAuth2 authentication for Gmail IMAP access using Google's "Sign in with Google" flow. This replaces username/password authentication which Gmail no longer supports as of March 2025.

## Background

Gmail requires OAuth 2.0 for third-party email client access via IMAP. The implementation uses:
- **OAuth2 Flow**: Authorization Code with PKCE (Proof Key for Code Exchange)
- **Security Model**: Client ID is public (embedded in app), security comes from PKCE and localhost redirect
- **Token Storage**: Refresh tokens stored in OS keyring, access tokens cached with expiry

## Technical Details

### OAuth2 Flow for Desktop Apps

1. User clicks "Add Gmail Account" in dwata UI
2. Backend generates PKCE code verifier and challenge
3. Backend opens user's browser to Google authorization URL
4. User grants permission in browser
5. Google redirects to `http://localhost:8080/api/oauth/google/callback?code=...`
6. Backend exchanges authorization code + PKCE verifier for tokens
7. Store refresh token in OS keyring
8. Use access token for IMAP authentication via XOAUTH2

### IMAP Authentication with OAuth2

Gmail IMAP uses SASL XOAUTH2 mechanism:
```
AUTHENTICATE XOAUTH2
base64("user={email}\x01auth=Bearer {access_token}\x01\x01")
```

Access tokens expire in ~1 hour, so refresh before each IMAP connection.

### Token Lifecycle

- **Refresh Token**: Long-lived, stored in OS keyring, never expires (unless revoked)
- **Access Token**: Short-lived (~1 hour), cached in memory with expiry timestamp
- **Token Refresh**: Use refresh token to get new access token when needed

## Architecture

```
┌─────────────────┐
│  dwata GUI      │ User clicks "Add Gmail Account"
└────────┬────────┘
         │
         ↓
┌─────────────────────────────────────────────────┐
│  POST /api/credentials/gmail/initiate           │
│  - Generate PKCE verifier + challenge           │
│  - Store verifier in memory (keyed by state)    │
│  - Return authorization URL to frontend         │
└────────┬────────────────────────────────────────┘
         │
         ↓
┌─────────────────┐
│  Open Browser   │ User grants permission
│  to Google Auth │
└────────┬────────┘
         │
         ↓
┌─────────────────────────────────────────────────┐
│  GET /api/oauth/google/callback?code=...&state=│
│  - Verify state parameter                       │
│  - Retrieve PKCE verifier from memory           │
│  - Exchange code + verifier for tokens          │
│  - Store refresh token in OS keyring            │
│  - Store access token in memory with expiry     │
│  - Store credential metadata in database        │
│  - Return success page or redirect to GUI       │
└────────┬────────────────────────────────────────┘
         │
         ↓
┌─────────────────────────────────────────────────┐
│  IMAP Connection                                 │
│  - Get access token from cache or refresh       │
│  - Authenticate using XOAUTH2 mechanism         │
└─────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Add Dependencies

**File**: `dwata-api/Cargo.toml`

Add to `[dependencies]`:
```toml
oauth2 = "4.4"
imap = "3.0"
native-tls = "0.2"
```

### Phase 2: Google OAuth2 Configuration

**File**: `dwata-api/src/config.rs`

Add OAuth2 configuration:
```rust
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl Default for GoogleOAuthConfig {
    fn default() -> Self {
        Self {
            // Replace with your Google Cloud Console client ID
            client_id: "YOUR_CLIENT_ID.apps.googleusercontent.com".to_string(),
            redirect_uri: "http://localhost:8080/api/oauth/google/callback".to_string(),
            scopes: vec!["https://mail.google.com/".to_string()],
        }
    }
}
```

**Note**: Client ID must be configured in Google Cloud Console with redirect URI `http://localhost:8080/api/oauth/google/callback`

### Phase 3: OAuth2 State Management

**File**: `dwata-api/src/helpers/oauth_state.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Temporary storage for OAuth2 PKCE verifiers during flow
pub struct OAuthStateManager {
    states: Arc<Mutex<HashMap<String, PkceVerifier>>>,
}

impl OAuthStateManager {
    pub fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn store_verifier(&self, state: String, verifier: oauth2::PkceCodeVerifier) {
        let mut states = self.states.lock().await;
        states.insert(state, verifier);
    }

    pub async fn retrieve_verifier(&self, state: &str) -> Option<oauth2::PkceCodeVerifier> {
        let mut states = self.states.lock().await;
        states.remove(state)
    }
}
```

### Phase 4: OAuth2 Client Setup

**File**: `dwata-api/src/helpers/google_oauth.rs`

```rust
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use oauth2::basic::BasicClient;
use anyhow::Result;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub struct GoogleOAuthClient {
    client: BasicClient,
}

impl GoogleOAuthClient {
    pub fn new(client_id: &str, redirect_uri: &str) -> Result<Self> {
        let client = BasicClient::new(
            ClientId::new(client_id.to_string()),
            None, // No client secret for public desktop apps
            AuthUrl::new(GOOGLE_AUTH_URL.to_string())?,
            Some(TokenUrl::new(GOOGLE_TOKEN_URL.to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())?);

        Ok(Self { client })
    }

    /// Generate authorization URL with PKCE
    pub fn authorize_url(&self) -> (String, CsrfToken, PkceCodeVerifier) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://mail.google.com/".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        (auth_url.to_string(), csrf_token, pkce_verifier)
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        code: String,
        pkce_verifier: PkceCodeVerifier,
    ) -> Result<oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>> {
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(oauth2::reqwest::async_http_client)
            .await?;

        Ok(token)
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>> {
        let token = self
            .client
            .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token.to_string()))
            .request_async(oauth2::reqwest::async_http_client)
            .await?;

        Ok(token)
    }
}
```

### Phase 5: Token Storage

**File**: `dwata-api/src/helpers/token_cache.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc, Duration};

#[derive(Clone)]
pub struct CachedToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

/// In-memory cache for access tokens
pub struct TokenCache {
    tokens: Arc<Mutex<HashMap<i64, CachedToken>>>, // credential_id -> token
}

impl TokenCache {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn store_token(&self, credential_id: i64, access_token: String, expires_in_seconds: i64) {
        let expires_at = Utc::now() + Duration::seconds(expires_in_seconds);
        let cached = CachedToken {
            access_token,
            expires_at,
        };

        let mut tokens = self.tokens.lock().await;
        tokens.insert(credential_id, cached);
    }

    pub async fn get_token(&self, credential_id: i64) -> Option<String> {
        let tokens = self.tokens.lock().await;
        if let Some(cached) = tokens.get(&credential_id) {
            if Utc::now() < cached.expires_at - Duration::minutes(5) {
                // Return if not expired (5 min buffer)
                return Some(cached.access_token.clone());
            }
        }
        None
    }

    pub async fn invalidate_token(&self, credential_id: i64) {
        let mut tokens = self.tokens.lock().await;
        tokens.remove(&credential_id);
    }
}
```

### Phase 6: OAuth2 Handlers

**File**: `dwata-api/src/handlers/oauth.rs`

```rust
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::helpers::google_oauth::GoogleOAuthClient;
use crate::helpers::oauth_state::OAuthStateManager;
use crate::helpers::keyring_service::KeyringService;
use crate::database::credentials as db;
use shared_types::credential::{CredentialType, CreateCredentialRequest};

#[derive(Serialize)]
pub struct InitiateOAuthResponse {
    pub authorization_url: String,
}

/// POST /api/credentials/gmail/initiate
/// Initiate Gmail OAuth2 flow
pub async fn initiate_gmail_oauth(
    oauth_client: web::Data<Arc<GoogleOAuthClient>>,
    state_manager: web::Data<Arc<OAuthStateManager>>,
) -> Result<HttpResponse> {
    let (auth_url, csrf_token, pkce_verifier) = oauth_client.authorize_url();

    // Store PKCE verifier temporarily (keyed by CSRF token)
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

/// GET /api/oauth/google/callback
/// OAuth2 callback endpoint (Google redirects here)
pub async fn google_oauth_callback(
    query: web::Query<OAuthCallbackQuery>,
    oauth_client: web::Data<Arc<GoogleOAuthClient>>,
    state_manager: web::Data<Arc<OAuthStateManager>>,
    db: web::Data<Arc<crate::database::Database>>,
    token_cache: web::Data<Arc<crate::helpers::token_cache::TokenCache>>,
) -> Result<HttpResponse> {
    // Check for error from Google
    if let Some(error) = &query.error {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("OAuth authorization failed: {}", error)
        })));
    }

    // Retrieve PKCE verifier
    let pkce_verifier = state_manager
        .retrieve_verifier(&query.state)
        .await
        .ok_or_else(|| {
            actix_web::error::ErrorBadRequest("Invalid state parameter")
        })?;

    // Exchange code for tokens
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

    // Get user email from access token (call Google's userinfo API)
    let email = get_user_email(access_token).await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to get user email: {}", e)))?;

    // Create credential in database
    let credential_request = CreateCredentialRequest {
        credential_type: CredentialType::OAuth,
        identifier: format!("gmail_{}", email.replace('@', "_at_")),
        username: email.clone(),
        password: refresh_token.clone(), // Store refresh token as "password"
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

    // Store refresh token in OS keyring
    KeyringService::set_password(
        &CredentialType::OAuth,
        &credential_request.identifier,
        &email,
        refresh_token,
    )
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to store refresh token: {}", e)))?;

    // Cache access token
    token_cache.store_token(metadata.id, access_token.to_string(), expires_in).await;

    // Return success page or redirect to GUI
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

/// Helper function to get user email from Google
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
```

### Phase 7: IMAP OAuth2 Authenticator

**File**: `dwata-api/src/helpers/imap_oauth.rs`

```rust
use imap::Authenticator;

/// XOAUTH2 authenticator for IMAP
pub struct XOAuth2 {
    user: String,
    access_token: String,
}

impl XOAuth2 {
    pub fn new(user: String, access_token: String) -> Self {
        Self { user, access_token }
    }
}

impl Authenticator for XOAuth2 {
    type Response = String;

    fn process(&self, _challenge: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

/// Get or refresh access token for Gmail IMAP
pub async fn get_access_token_for_imap(
    credential_id: i64,
    credential: &shared_types::credential::CredentialMetadata,
    token_cache: &crate::helpers::token_cache::TokenCache,
    oauth_client: &crate::helpers::google_oauth::GoogleOAuthClient,
    keyring_service: &crate::helpers::keyring_service::KeyringService,
) -> anyhow::Result<String> {
    // Try to get from cache
    if let Some(access_token) = token_cache.get_token(credential_id).await {
        return Ok(access_token);
    }

    // Refresh token
    let refresh_token = KeyringService::get_password(
        &credential.credential_type,
        &credential.identifier,
        &credential.username,
    )?;

    let token_response = oauth_client.refresh_token(&refresh_token).await?;
    let access_token = token_response.access_token().secret().to_string();
    let expires_in = token_response.expires_in().unwrap_or_default().as_secs() as i64;

    // Cache the new access token
    token_cache.store_token(credential_id, access_token.clone(), expires_in).await;

    Ok(access_token)
}
```

### Phase 8: Update IMAP Client

**File**: `dwata-api/src/integrations/imap_client.rs` (create new or update existing)

```rust
use imap::Session;
use native_tls::TlsConnector;
use anyhow::Result;

use crate::helpers::imap_oauth::{XOAuth2, get_access_token_for_imap};

pub async fn connect_gmail_oauth(
    email: &str,
    credential_id: i64,
    credential: &shared_types::credential::CredentialMetadata,
    token_cache: &crate::helpers::token_cache::TokenCache,
    oauth_client: &crate::helpers::google_oauth::GoogleOAuthClient,
) -> Result<Session<Box<dyn std::io::Read + std::io::Write + Send>>> {
    // Get or refresh access token
    let access_token = get_access_token_for_imap(
        credential_id,
        credential,
        token_cache,
        oauth_client,
        &crate::helpers::keyring_service::KeyringService,
    )
    .await?;

    // Connect to Gmail IMAP
    let tls = TlsConnector::builder().build()?;
    let client = imap::connect(("imap.gmail.com", 993), "imap.gmail.com", &tls)?;

    // Authenticate using XOAUTH2
    let authenticator = XOAuth2::new(email.to_string(), access_token);
    let session = client.authenticate("XOAUTH2", &authenticator)
        .map_err(|(err, _)| err)?;

    Ok(session)
}
```

### Phase 9: Route Registration

**File**: `dwata-api/src/main.rs`

Add to route setup:
```rust
use crate::helpers::{
    google_oauth::GoogleOAuthClient,
    oauth_state::OAuthStateManager,
    token_cache::TokenCache,
};

// Initialize OAuth components
let oauth_client = Arc::new(GoogleOAuthClient::new(
    &config.google_oauth.client_id,
    &config.google_oauth.redirect_uri,
)?);
let state_manager = Arc::new(OAuthStateManager::new());
let token_cache = Arc::new(TokenCache::new());

// Add to app data
.app_data(web::Data::new(oauth_client.clone()))
.app_data(web::Data::new(state_manager.clone()))
.app_data(web::Data::new(token_cache.clone()))

// Add routes
.route("/api/credentials/gmail/initiate", web::post().to(handlers::oauth::initiate_gmail_oauth))
.route("/api/oauth/google/callback", web::get().to(handlers::oauth::google_oauth_callback))
```

### Phase 10: Update Download Manager

**File**: `dwata-api/src/jobs/download_manager.rs`

Update to use OAuth2 for Gmail credentials:
```rust
// In run_imap_download function
let credential = get_credential(db_conn.clone(), job.credential_id).await?;

let imap_session = if credential.credential_type == CredentialType::OAuth {
    // OAuth2 flow (Gmail)
    connect_gmail_oauth(
        &credential.username,
        credential.id,
        &credential,
        &token_cache,
        &oauth_client,
    ).await?
} else {
    // Traditional username/password (other providers)
    let password = KeyringService::get_password(
        &credential.credential_type,
        &credential.identifier,
        &credential.username,
    )?;
    // ... existing IMAP connection code
};
```

## Google Cloud Console Setup

Before implementation, set up OAuth2 credentials:

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing
3. Enable Gmail API:
   - APIs & Services → Library → Search "Gmail API" → Enable
4. Create OAuth2 credentials:
   - APIs & Services → Credentials → Create Credentials → OAuth client ID
   - Application type: **Desktop app**
   - Name: "dwata Desktop App"
   - Click Create
5. Note the **Client ID** (no client secret needed for desktop apps)
6. Configure OAuth consent screen:
   - User type: External (or Internal if G Workspace)
   - Add test users during development
   - Scopes: Add `https://mail.google.com/` (Full IMAP/SMTP access)
7. Add authorized redirect URI:
   - `http://localhost:8080/api/oauth/google/callback`

## Frontend Integration

The frontend needs to:

1. **Initiate OAuth Flow**:
```typescript
// User clicks "Add Gmail Account"
const response = await fetch('http://localhost:8080/api/credentials/gmail/initiate', {
  method: 'POST'
});
const { authorization_url } = await response.json();

// Open browser to authorization URL
window.open(authorization_url, '_blank');
```

2. **Handle Success**:
The OAuth callback endpoint will close the browser window automatically. The frontend should poll for new credentials or use WebSocket for real-time updates.

## Testing

### Manual Test Flow

1. Start dwata-api: `cargo run`
2. Initiate OAuth:
```bash
curl -X POST http://localhost:8080/api/credentials/gmail/initiate
```
3. Copy `authorization_url` and open in browser
4. Grant permission
5. Verify redirect to callback endpoint
6. Check credential created in database:
```bash
curl http://localhost:8080/api/credentials
```
7. Verify refresh token in OS keyring (macOS):
```bash
security find-generic-password -s "dwata:oauth" -a "gmail_*" -w
```

### Test IMAP Connection

After credential is created, test IMAP connection in download manager to ensure OAuth2 authentication works.

## Success Criteria

- [ ] User can click "Add Gmail Account" and complete OAuth2 flow
- [ ] Refresh token stored securely in OS keyring
- [ ] Access token cached with expiry
- [ ] IMAP connection works with OAuth2 access token
- [ ] Access token automatically refreshed when expired
- [ ] Credential metadata stored in database with type=OAuth
- [ ] No client secret embedded in code (not needed for public clients)

## Security Notes

- **Client ID is public**: Embedded in app, this is expected and safe
- **PKCE provides security**: Prevents authorization code interception
- **Localhost redirect**: Ensures only local app receives code
- **Refresh token in keyring**: Long-lived token stored securely
- **Access token in memory**: Short-lived, not persisted to disk

## Documentation

After implementation, update README with:
- Google Cloud Console setup instructions
- How to configure client ID
- OAuth2 flow explanation for users
- Troubleshooting common OAuth issues
