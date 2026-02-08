use native_tls::TlsConnector;
use anyhow::Result;

use crate::helpers::imap_oauth::get_access_token_for_imap;

pub async fn connect_gmail_oauth(
    _email: &str,
    _credential_id: i64,
    _credential: &shared_types::credential::CredentialMetadata,
    _token_cache: &crate::helpers::token_cache::TokenCache,
    _oauth_client: &crate::helpers::google_oauth::GoogleOAuthClient,
    _keyring_service: &crate::helpers::keyring_service::KeyringService,
) -> Result<()> {
    let _access_token = get_access_token_for_imap(
        _credential_id,
        _credential,
        _token_cache,
        _oauth_client,
        _keyring_service,
    )
    .await?;

    let _tls = TlsConnector::builder().build()?;

    Ok(())
}
