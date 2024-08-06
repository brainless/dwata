use crate::error::DwataError;
use crate::workspace::crud::CRUDCreateUpdate;
use chrono::{DateTime, Utc};
use helpers::get_google_oauth2_client;
use log::{error, info};
use oauth2::{
    basic::BasicTokenType, reqwest::async_http_client, AuthorizationCode, EmptyExtraTokenFields,
    RefreshToken, StandardTokenResponse, TokenResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, FromRow, Pool, Sqlite, Type};
use strum::{Display, EnumString};
use ts_rs::TS;

pub mod api;
pub mod commands;
pub mod crud;
pub mod helpers;

#[derive(Debug, Deserialize, Serialize, Clone, TS, Type, EnumString, Display)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[ts(export)]
pub enum OAuth2Provider {
    Google,
}

#[derive(Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OAuth2App {
    #[ts(type = "number")]
    pub id: i64,

    pub provider: OAuth2Provider,
    pub client_id: String,
    pub client_secret: Option<String>,

    pub created_at: DateTime<Utc>,
    pub modified_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OAuth2Token {
    #[ts(type = "number")]
    pub id: i64,
    pub oauth2_app_id: i64,

    pub authorization_code: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    // Unique id sent from provider
    pub identifier: String,
    // Email address, username, etc.
    pub handle: Option<String>,

    pub created_at: DateTime<Utc>,
    pub modified_at: Option<DateTime<Utc>>,

    #[serde(skip)]
    #[sqlx(skip)]
    #[ts(skip)]
    pub oauth2_app: Option<OAuth2App>,
}

#[derive(Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OAuth2AppCreateUpdate {
    pub provider: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Default)]
pub struct OAuth2TokenCreateUpdate {
    pub oauth2_app_id: Option<i64>,
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
}

pub struct Oauth2APIResponse {
    pub authorization_code: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub identifier: String,
    pub handle: Option<String>,
}

impl OAuth2Token {
    pub async fn get_oauth2_app(&mut self, db: &Pool<Sqlite>) -> Result<OAuth2App, DwataError> {
        if let Some(oauth2_app) = &self.oauth2_app {
            Ok(oauth2_app.clone())
        } else {
            let sql = "SELECT * FROM oauth2_app WHERE id = ?";
            let oauth2_app: OAuth2App =
                query_as(sql).bind(self.oauth2_app_id).fetch_one(db).await?;
            self.oauth2_app = Some(oauth2_app);
            Ok(self.oauth2_app.as_ref().unwrap().clone())
        }
    }

    pub async fn refetch_google_access_token(
        &mut self,
        db: &Pool<Sqlite>,
    ) -> Result<(), DwataError> {
        let oauth2_app = self.get_oauth2_app(db).await?;
        let client = get_google_oauth2_client(
            oauth2_app.client_id.clone(),
            oauth2_app.client_secret.as_ref().unwrap().clone(),
        )?;

        let mut token_response: Option<
            StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
        > = None;
        // If we have a refresh token, then we can use it to get a new access token
        match self.refresh_token.as_ref() {
            Some(x) => {
                let refresh_token = RefreshToken::new(x.clone());
                token_response = match client
                    .exchange_refresh_token(&refresh_token)
                    .request_async(&async_http_client)
                    .await
                {
                    Ok(x) => Some(x),
                    Err(err) => {
                        error!("Could not refresh access token\n Error: {}", err);
                        None
                    }
                };
            }
            None => {
                info!("No refresh token found");
            }
        }

        let oauth2_token: OAuth2Token = {
            let sql = "SELECT * FROM oauth2_token WHERE oauth2_app_id = ?";
            query_as(sql).bind(self.id).fetch_one(db).await?
        };
        match token_response {
            Some(x) => {
                let updated = OAuth2TokenCreateUpdate {
                    oauth2_app_id: Some(self.id),
                    refresh_token: x.refresh_token().map(|x| x.secret().clone()),
                    access_token: Some(x.access_token().secret().to_string()),
                };
                updated.update_module_data(oauth2_token.id, db).await?;
            }
            None => {
                let code = AuthorizationCode::new(self.authorization_code.clone());
                // Exchange the authorization code with a token
                token_response = match client
                    .exchange_code(code)
                    // .set_pkce_verifier(pkce_code_verifier)
                    .request_async(&async_http_client)
                    .await
                {
                    Ok(x) => Some(x),
                    Err(err) => {
                        error!("Could not exchange code\n Error: {}", err);
                        return Err(DwataError::CouldNotGetTokenResponse);
                    }
                };

                match token_response {
                    Some(x) => {
                        let updated = OAuth2TokenCreateUpdate {
                            oauth2_app_id: Some(self.id),
                            refresh_token: x.refresh_token().map(|x| x.secret().clone()),
                            access_token: Some(x.access_token().secret().to_string()),
                        };
                        updated.update_module_data(self.id, db).await?;
                    }
                    None => {
                        error!("Could not get token response");
                        return Err(DwataError::CouldNotGetTokenResponse);
                    }
                }
            }
        }

        Ok(())
    }
}
