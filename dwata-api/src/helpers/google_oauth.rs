use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenUrl,
};
use oauth2::basic::BasicClient;
use anyhow::Result;
use std::time::Duration;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub struct GoogleOAuthClient {
    client: BasicClient,
    http_client: reqwest::Client,
}

impl GoogleOAuthClient {
    fn async_http_client(&self) -> impl Fn(oauth2::HttpRequest) -> futures::future::BoxFuture<'static, Result<oauth2::HttpResponse, reqwest::Error>> {
        let client = self.http_client.clone();
        move |request: oauth2::HttpRequest| {
            let client = client.clone();
            Box::pin(async move {
                let request_builder = match request.method {
                    oauth2::http::Method::GET => client.get(request.url.as_str()),
                    oauth2::http::Method::POST => client.post(request.url.as_str()),
                    oauth2::http::Method::PUT => client.put(request.url.as_str()),
                    oauth2::http::Method::DELETE => client.delete(request.url.as_str()),
                    _ => unimplemented!("Unsupported HTTP method"),
                };
                let request_builder = request_builder.headers(request.headers);
                let request_builder = request_builder.body(request.body);
                let response = request_builder.send().await?;
                Ok(oauth2::HttpResponse {
                    status_code: response.status().as_u16().try_into().unwrap(),
                    headers: response.headers().clone(),
                    body: response.bytes().await?.to_vec(),
                })
            })
        }
    }
}

impl GoogleOAuthClient {
    pub fn new(client_id: &str, client_secret: Option<&str>, redirect_uri: &str) -> Result<Self> {
        let client = BasicClient::new(
            ClientId::new(client_id.to_string()),
            client_secret.map(|s| ClientSecret::new(s.to_string())),
            AuthUrl::new(GOOGLE_AUTH_URL.to_string())?,
            Some(TokenUrl::new(GOOGLE_TOKEN_URL.to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())?);

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        Ok(Self { client, http_client })
    }

    pub fn authorize_url(&self) -> (String, CsrfToken, PkceCodeVerifier) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (mut auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://mail.google.com/".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        // Add access_type=offline to get refresh token
        auth_url.query_pairs_mut()
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");

        (auth_url.to_string(), csrf_token, pkce_verifier)
    }

    pub async fn exchange_code(
        &self,
        code: String,
        pkce_verifier: PkceCodeVerifier,
    ) -> Result<oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>> {
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(self.async_http_client())
            .await?;

        Ok(token)
    }

    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>> {
        tracing::debug!("Attempting to refresh OAuth token");
        let token = self
            .client
            .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token.to_string()))
            .request_async(self.async_http_client())
            .await
            .map_err(|e| {
                tracing::warn!("OAuth token refresh failed: {}", e);
                anyhow::anyhow!("Failed to refresh OAuth token: {}", e)
            })?;

        tracing::debug!("OAuth token refreshed successfully");
        Ok(token)
    }
}
