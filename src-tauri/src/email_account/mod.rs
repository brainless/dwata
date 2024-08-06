use chrono::{DateTime, Utc};
use imap::Session;
use native_tls::TlsStream;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use strum::{Display, EnumString};
use tokio::sync::Mutex;
use ts_rs::TS;

pub mod api;
pub mod app_state;
pub mod commands;
pub mod crud;
pub mod helpers;
pub mod providers;

#[derive(Deserialize, Serialize, Type, TS, EnumString, Display)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[ts(export)]
pub enum EmailProvider {
    Gmail,
    ProtonMail,
    // Yahoo,
    // Outlook,
}

pub struct EmailAccountStatus {
    pub id: i64,
    // IMAP protocol and local storage related
    pub mailbox: Mailbox,
    pub storage_dir: PathBuf,
    // Related to IMAP processing in dwata
    pub last_fetched_at: Option<i64>,
    // Which date range is being searched
    // pub searching_date_range: Option<(NaiveDate, NaiveDate)>,
    pub imap_session: Option<Arc<Mutex<Session<TlsStream<TcpStream>>>>>,
}

#[derive(Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct EmailAccount {
    #[ts(type = "number")]
    pub id: i64,

    pub provider: EmailProvider,

    pub email_address: String,
    pub password: Option<String>,
    pub oauth2_token_id: Option<i64>,

    pub created_at: DateTime<Utc>,
    pub modified_at: Option<DateTime<Utc>>,
}

#[derive(Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct EmailAccountCreateUpdate {
    pub provider: Option<String>,
    pub email_address: Option<String>,
    pub password: Option<String>,
    pub oauth2_app_id: Option<i64>,
    pub oauth2_token_id: Option<i64>,
}

#[derive(Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Mailbox {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub email_account_id: i64,
    pub name: String,
    #[ts(skip)]
    pub storage_path: String,

    messages: Option<u32>,
    uid_next: Option<u32>,
    uid_validity: Option<u32>,
}

#[derive(Default)]
pub struct MailboxCreateUpdate {
    pub email_account_id: Option<i64>,
    pub name: Option<String>,
    pub storage_path: Option<String>,
    pub messages: Option<u32>,
    pub uid_next: Option<u32>,
    pub uid_validity: Option<u32>,
}
