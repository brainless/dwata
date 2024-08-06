use super::app_state::EmailAccountsState;
use super::{EmailAccount, EmailProvider, Mailbox};
use crate::error::DwataError;
use crate::oauth2::OAuth2Token;
use crate::workspace::crud::CRUDRead;
use imap::Session;
use log::{error, info};
use native_tls::{TlsConnector, TlsStream};
use sqlx::{Pool, Sqlite};
use std::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct GmailAccount {
    email_address: String,
    access_token: String,
}

impl imap::Authenticator for GmailAccount {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.email_address, self.access_token
        )
    }
}

impl EmailAccount {
    pub async fn create_imap_session(
        &mut self,
        db: &Pool<Sqlite>,
        email_account_state: &EmailAccountsState,
    ) -> Result<Arc<Mutex<Session<TlsStream<TcpStream>>>>, DwataError> {
        let mut session: Option<Session<TlsStream<TcpStream>>> = None;
        match self.provider {
            EmailProvider::Gmail => {
                let mut tries = 0;
                while tries < 2 {
                    tries += 1;
                    let mut oauth2_token = self.get_oauth2_token_for_gmail(db).await?;
                    let gmail_account = GmailAccount {
                        email_address: self.email_address.clone(),
                        access_token: oauth2_token.access_token.clone(),
                    };
                    let domain = "imap.gmail.com";
                    let tls = TlsConnector::builder().build().unwrap();
                    let client = imap::connect((domain, 993), domain, &tls).unwrap();

                    match client.authenticate("XOAUTH2", &gmail_account) {
                        Ok(s) => {
                            info!("Logged into Gmail with Oauth2 credentials");
                            session = Some(s);
                            break;
                        }
                        Err((error, _)) => {
                            error!("Error authenticating with Gmail: {}", error);
                            oauth2_token.refetch_google_access_token(db).await?;
                        }
                    }
                }
                if tries == 2 {
                    return Err(DwataError::CouldNotAuthenticateToService);
                }
            }
            EmailProvider::ProtonMail => {
                let domain = "127.0.0.1";
                // This is very dangerous and only needed for ProtonMail Bridge, where connection is ending to localhost
                let tls = TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .unwrap();
                let client = imap::connect_starttls((domain, 1143), domain, &tls).unwrap();

                match client.login(&self.email_address, &self.password.as_ref().unwrap()) {
                    Ok(s) => {
                        info!("Logged into Protonmail with username/password");
                        session = Some(s);
                    }
                    Err((error, _unauth_client)) => {
                        error!("Error authenticating with Protonmail:\n{}", error);
                        return Err(DwataError::CouldNotAuthenticateToService);
                    }
                }
            }
        };

        match session {
            Some(s) => {
                let session = Arc::new(Mutex::new(s));
                let mut locked_state = email_account_state.lock().await;
                // Filter the vec of EmailAccountStatus to find the one that matches this EmailAccount
                match locked_state.iter_mut().find(|x| x.id == self.id) {
                    Some(x) => {
                        x.imap_session = Some(session.clone());
                        Ok(session)
                    }
                    None => Err(DwataError::AppStateNotFound),
                }
            }
            None => {
                error!("Could not authenticate to IMAP server");
                Err(DwataError::CouldNotAuthenticateToService)
            }
        }
    }

    pub async fn get_imap_session(
        &mut self,
        db: &Pool<Sqlite>,
        email_account_state: &EmailAccountsState,
        create_if_not_found: bool,
    ) -> Result<Arc<Mutex<Session<TlsStream<TcpStream>>>>, DwataError> {
        let session = {
            let locked_state = email_account_state.lock().await;
            // Filter the vec of EmailAccountStatus to find the one that matches this EmailAccount
            match locked_state.iter().find(|x| x.id == self.id) {
                Some(x) => x.imap_session.clone(),
                None => None,
            }
        };
        if let Some(session) = session {
            Ok(session.clone())
        } else if create_if_not_found {
            self.create_imap_session(db, email_account_state).await
        } else {
            Err(DwataError::AppStateNotFound)
        }
    }

    pub async fn get_oauth2_token_for_gmail(
        &self,
        db: &Pool<Sqlite>,
    ) -> Result<OAuth2Token, DwataError> {
        if let Some(oauth2_token_id) = self.oauth2_token_id {
            OAuth2Token::read_one_by_pk(oauth2_token_id, db).await
        } else {
            Err(DwataError::CouldNotFindOAuth2Config)
        }
    }

    pub async fn get_selected_mailbox(
        &self,
        _db: &Pool<Sqlite>,
        email_account_state: &EmailAccountsState,
    ) -> Result<Mailbox, DwataError> {
        let email_account_state = email_account_state.lock().await;
        // Filter the vec of EmailAccountStatus to find the one that matches this EmailAccount
        match email_account_state.iter().find(|x| x.id == self.id) {
            Some(x) => Ok(x.mailbox.clone()),
            None => Err(DwataError::AppStateNotFound),
        }
    }
}
