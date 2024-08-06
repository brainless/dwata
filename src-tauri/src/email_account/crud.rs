use super::{EmailAccount, EmailAccountCreateUpdate, EmailProvider, Mailbox, MailboxCreateUpdate};
use crate::content::content::{Content, ContentType};
use crate::error::DwataError;
use crate::oauth2::{OAuth2Token, OAuth2TokenCreateUpdate};
use crate::workspace::crud::{
    CRUDCreateUpdate, CRUDRead, InputValue, InsertUpdateResponse, VecColumnNameValue,
};
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

impl CRUDRead for EmailAccount {
    fn table_name() -> String {
        "email_account".to_string()
    }
}

impl CRUDCreateUpdate for EmailAccountCreateUpdate {
    fn table_name() -> String {
        "email_account".to_string()
    }

    fn get_column_names_values(&self) -> Result<VecColumnNameValue, DwataError> {
        let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.provider {
            names_values.push_name_value("provider", InputValue::Text(x.clone()));
            let provider = match EmailProvider::from_str(&x) {
                Ok(x) => x,
                Err(_) => return Err(DwataError::InvalidEmailProvider),
            };
            match provider {
                EmailProvider::Gmail => {
                    // The OAuth2 ID will be checked from DB in pre_insert and then added
                }
                EmailProvider::ProtonMail => {
                    if let Some(x) = &self.email_address {
                        names_values.push_name_value("email_address", InputValue::Text(x.clone()));
                    }
                    if let Some(x) = &self.password {
                        names_values.push_name_value("password", InputValue::Text(x.clone()));
                    }
                }
            }
        }

        names_values.push_name_value("created_at", InputValue::DateTime(Utc::now()));
        Ok(names_values)
    }

    async fn pre_insert(&self, db: &Pool<Sqlite>) -> Result<VecColumnNameValue, DwataError> {
        // We check if gmail is the provider and if so, we create the OAuth2 token for the OAuth2 app
        // In the OAuth2CreateUpdate, there is a pre_insert function that will call the OAuth2 server
        // and get the authorization code, refresh token, access token, identifier etc.
        let mut name_values: VecColumnNameValue = VecColumnNameValue::default();
        if self.provider == Some("gmail".to_string()) {
            if self.oauth2_app_id.is_none() {
                return Err(DwataError::CouldNotFindOAuth2Config);
            }
            let oauth2_token_id = OAuth2TokenCreateUpdate {
                oauth2_app_id: Some(self.oauth2_app_id.unwrap()),
                ..Default::default()
            }
            .insert_module_data(db)
            .await?
            .pk;
            match OAuth2Token::read_one_by_pk(oauth2_token_id, db).await {
                Ok(oauth2_token) => {
                    name_values.push_name_value("oauth2_token_id", InputValue::ID(oauth2_token.id));
                    name_values.push_name_value(
                        "email_address",
                        InputValue::Text(oauth2_token.handle.unwrap()),
                    );
                }
                Err(_) => return Err(DwataError::CouldNotFindOAuth2Config),
            };
        }
        Ok(name_values)
    }

    async fn post_insert(
        &self,
        response: InsertUpdateResponse,
        _db: &Pool<Sqlite>,
    ) -> Result<InsertUpdateResponse, DwataError> {
        // When we add a new Email Account, we want to fetch emails from that account
        // We do this by calling the fetch_emails function as the next step
        Ok(InsertUpdateResponse {
            next_task: Some("fetch_emails".to_string()),
            arguments: Some(vec![(
                "pk".to_string(),
                ContentType::ID,
                Content::ID(response.pk),
            )]),
            ..response
        })
    }
}

impl CRUDRead for Mailbox {
    fn table_name() -> String {
        "mailbox".to_string()
    }
}

impl CRUDCreateUpdate for MailboxCreateUpdate {
    fn table_name() -> String {
        "mailbox".to_string()
    }

    fn get_column_names_values(&self) -> Result<VecColumnNameValue, DwataError> {
        let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.email_account_id {
            names_values.push_name_value("email_account_id", InputValue::ID(*x));
        }
        if let Some(x) = &self.name {
            names_values.push_name_value("name", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.storage_path {
            names_values.push_name_value("storage_path", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.messages {
            names_values.push_name_value("messages", InputValue::ID(i64::from(*x)));
        }
        if let Some(x) = &self.uid_next {
            names_values.push_name_value("uid_next", InputValue::ID(i64::from(*x)));
        }
        if let Some(x) = &self.uid_validity {
            names_values.push_name_value("uid_validity", InputValue::ID(i64::from(*x)));
        }

        names_values.push_name_value("created_at", InputValue::DateTime(Utc::now()));
        Ok(names_values)
    }
}
