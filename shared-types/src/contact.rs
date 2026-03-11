use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Contact {
    pub id: i64,
    pub email_id: Option<i64>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateContactRequest {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateContactRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ContactsResponse {
    pub contacts: Vec<Contact>,
}
