use super::Oauth2APIResponse;
use crate::error::DwataError;
use log::error;
use oauth2::reqwest::async_http_client;
use oauth2::{basic::BasicClient, TokenResponse};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, RequestTokenError,
    Scope, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: String,
}

pub fn get_google_oauth2_client(
    client_id: String,
    client_secret: String,
) -> Result<BasicClient, DwataError> {
    // Set up the config for the Google OAuth2 process.
    let auth_url = match AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()) {
        Ok(auth_url) => auth_url,
        Err(err) => {
            error!("Could not create auth URL\n Error: {}", err);
            return Err(DwataError::CouldNotCreateAuthURL);
        }
    };
    let token_url = match TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string()) {
        Ok(token_url) => token_url,
        Err(err) => {
            error!("Could not create token URL\n Error: {}", err);
            return Err(DwataError::CouldNotCreateTokenURL);
        }
    };

    // This will be running its own server at localhost:8080.
    // See below for the server implementation.
    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(
        RedirectUrl::new("http://localhost:8080".to_string()).expect("Invalid redirect URL"),
    );

    Ok(client)
}

pub async fn get_google_oauth2_authorize_url(
    client_id: String,
    client_secret: String,
) -> Result<String, DwataError> {
    let client = get_google_oauth2_client(client_id, client_secret)?;

    // Google supports Proof Key for Code Exchange (PKCE - https://oauth.net/2/pkce/).
    // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
    // let (pkce_code_challenge, _pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the authorization URL to which we'll redirect the user.
    let (authorize_url, _csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        // This example is requesting access to the "calendar" features and the user's profile.
        .add_scope(Scope::new("https://mail.google.com/".to_string()))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/userinfo.email".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/userinfo.profile".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/plus.me".to_string(),
        ))
        // .set_pkce_challenge(pkce_code_challenge)
        .url();

    Ok(authorize_url.to_string())
}

pub async fn get_google_oauth2_tokens(
    client_id: String,
    client_secret: String,
) -> Result<Oauth2APIResponse, DwataError> {
    let client = get_google_oauth2_client(client_id, client_secret)?;

    // Google supports Proof Key for Code Exchange (PKCE - https://oauth.net/2/pkce/).
    // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
    // let (_pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    let (code, _state) = {
        // A very naive implementation of the redirect server.
        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

        // The server will terminate itself after collecting the first code.
        let Some(mut stream) = listener.incoming().flatten().next() else {
            panic!("listener terminated without accepting a connection");
        };

        let mut reader = BufReader::new(&stream);

        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();

        let redirect_url = request_line.split_whitespace().nth(1).unwrap();
        let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
            .unwrap();

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, state)| CsrfToken::new(state.into_owned()))
            .unwrap();

        let message = "You may close this window or tab";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
            message.len(),
            message
        );
        stream.write_all(response.as_bytes()).unwrap();

        (code, state)
    };

    let auth_code = format!("{}", code.secret());
    // Exchange the authorization code with a token
    let token_response = match client
        .exchange_code(code)
        // .set_pkce_verifier(pkce_code_verifier)
        .request_async(&async_http_client)
        .await
    {
        Ok(token_response) => token_response,
        Err(err) => {
            error!("Could not exchange code\n Error: {}", err);
            return Err(DwataError::CouldNotGetTokenResponse);
        }
    };

    let user_info = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v1/userinfo")
        .header(
            "Authorization",
            format!("Bearer {}", token_response.access_token().secret()),
        )
        .send()
        .await?
        .json::<GoogleUserInfo>()
        .await?;

    Ok(Oauth2APIResponse {
        authorization_code: auth_code,
        access_token: token_response.access_token().secret().clone(),
        refresh_token: token_response.refresh_token().map(|x| x.secret().clone()),
        identifier: user_info.id,
        handle: Some(user_info.email),
    })
}
