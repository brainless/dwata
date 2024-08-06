use super::{
    helpers::get_google_oauth2_tokens, OAuth2App, OAuth2AppCreateUpdate, OAuth2Token,
    OAuth2TokenCreateUpdate,
};
use crate::{
    error::DwataError,
    workspace::crud::{CRUDCreateUpdate, CRUDRead, InputValue, VecColumnNameValue},
};
use chrono::Utc;
use sqlx::{query_as, Pool, Sqlite};

impl CRUDRead for OAuth2App {
    fn table_name() -> String {
        "oauth2_app".to_string()
    }
}

impl CRUDCreateUpdate for OAuth2AppCreateUpdate {
    fn table_name() -> String {
        "oauth2_app".to_string()
    }

    fn get_column_names_values(&self) -> Result<VecColumnNameValue, DwataError> {
        let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.provider {
            names_values.push_name_value("provider", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.client_id {
            names_values.push_name_value("client_id", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.client_secret {
            names_values.push_name_value("client_secret", InputValue::Text(x.clone()));
        }
        names_values.push_name_value("created_at", InputValue::DateTime(Utc::now()));
        Ok(names_values)
    }
}

impl CRUDRead for OAuth2Token {
    fn table_name() -> String {
        "oauth2_token".to_string()
    }
}

impl CRUDCreateUpdate for OAuth2TokenCreateUpdate {
    fn table_name() -> String {
        "oauth2_token".to_string()
    }

    fn get_column_names_values(&self) -> Result<VecColumnNameValue, DwataError> {
        let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.oauth2_app_id {
            names_values.push_name_value("oauth2_app_id", InputValue::ID(*x));
        }
        if let Some(x) = &self.refresh_token {
            names_values.push_name_value("refresh_token", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.access_token {
            names_values.push_name_value("access_token", InputValue::Text(x.clone()));
        }
        names_values.push_name_value("created_at", InputValue::DateTime(Utc::now()));
        Ok(names_values)
    }

    async fn pre_insert(&self, db: &Pool<Sqlite>) -> Result<VecColumnNameValue, DwataError> {
        // We call the OAuth2 server and get the authorization code,
        // refresh token, access token, identifier etc.
        if self.oauth2_app_id.is_none() {
            return Err(DwataError::CouldNotFindOAuth2Config);
        }
        let sql = "SELECT * FROM oauth2_app WHERE id = ?";
        let oauth2_app: OAuth2App = query_as(sql).bind(self.oauth2_app_id).fetch_one(db).await?;
        let oauth2_response = get_google_oauth2_tokens(
            oauth2_app.client_id.clone(),
            oauth2_app.client_secret.as_ref().unwrap().clone(),
        )
        .await?;
        let mut name_values: VecColumnNameValue = VecColumnNameValue::default();
        name_values.push_name_value(
            "authorization_code",
            InputValue::Text(oauth2_response.authorization_code),
        );
        name_values.push_name_value(
            "access_token",
            InputValue::Text(oauth2_response.access_token),
        );
        name_values.push_name_value(
            "refresh_token",
            InputValue::Text(oauth2_response.refresh_token.unwrap_or_default()),
        );
        name_values.push_name_value("identifier", InputValue::Text(oauth2_response.identifier));
        if let Some(x) = oauth2_response.handle {
            name_values.push_name_value("handle", InputValue::Text(x.clone()));
        }
        Ok(name_values)
    }
}
