use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenUrl,
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
            None,
            AuthUrl::new(GOOGLE_AUTH_URL.to_string())?,
            Some(TokenUrl::new(GOOGLE_TOKEN_URL.to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())?);

        Ok(Self { client })
    }

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
