use anyhow::Result;
use oauth2::TokenResponse;

pub struct XOAuth2 {
    user: String,
    access_token: String,
}

impl XOAuth2 {
    pub fn new(user: String, access_token: String) -> Self {
        Self { user, access_token }
    }
}

impl imap::Authenticator for XOAuth2 {
    type Response = String;

    fn process(&self, _challenge: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

pub async fn get_access_token_for_imap(
    credential_id: i64,
    credential: &shared_types::credential::CredentialMetadata,
    token_cache: &crate::helpers::token_cache::TokenCache,
    oauth_client: &crate::helpers::google_oauth::GoogleOAuthClient,
    keyring_service: &crate::helpers::keyring_service::KeyringService,
) -> Result<String> {
    if let Some(access_token) = token_cache.get_token(credential_id).await {
        return Ok(access_token);
    }

    let refresh_token = keyring_service
        .get_password(
            &credential.credential_type,
            &credential.identifier,
            &credential.username,
        )
        .await?;

    let token_response = oauth_client.refresh_token(&refresh_token).await?;
    let access_token = token_response.access_token().secret().to_string();
    let expires_in = token_response.expires_in().unwrap_or_default().as_secs() as i64;

    token_cache.store_token(credential_id, access_token.clone(), expires_in).await;

    Ok(access_token)
}
