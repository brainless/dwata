use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod commands;
pub mod crud;
pub mod helpers;

#[derive(Default, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct Email {
    // The Uid of the email in the mailbox
    #[ts(type = "number")]
    pub uid: u32,
    #[ts(type = "number")]
    pub mailbox_id: i64,

    pub from_name: String,
    pub from_email: String,
    pub date: i64,
    pub subject: String,

    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct EmailFull {
    pub email: Email,
    pub body_text: String,
}

#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct EmailFilters {
    #[ts(type = "number")]
    pub mailbox_id: Option<i64>,
    // pub from_name: Option<String>,
    // pub from_email: Option<String>,
    // #[ts(type = "number")]
    // pub date_from: Option<i64>,
    // #[ts(type = "number")]
    // pub date_to: Option<i64>,
    // pub subject: Option<String>,
    pub search_query: Option<String>,
}

#[derive(Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct EmailBucket {
    #[ts(type = "number")]
    pub id: i64,
    // This is generally the root of a thread, can be null for buckets of similar emails, etc.
    #[ts(type = "number")]
    pub root_email_uid: Option<u32>,

    #[ts(type = "Array<number>")]
    pub uid_list: Vec<u32>,

    pub summary: Option<String>,

    // If this is a bucket of emails in a thread
    pub are_in_a_thread: Option<bool>,

    // If this is a bucket of similar emails, like bank notifications of same kind
    pub are_similar: Option<bool>,
    // If this is a bucket of emails from a contact
    // pub contact_id: Option<i64>,

    // If this is a bucket of emails with a label
    // pub label_id: Option<i64>,
}
