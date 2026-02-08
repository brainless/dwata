use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ContactLinkType {
    Linkedin,
    Github,
    Twitter,
    Personal,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ContactLink {
    pub id: i64,
    pub contact_id: i64,
    pub link_type: ContactLinkType,
    pub url: String,
    pub label: Option<String>,
    pub is_primary: bool,
    pub is_verified: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateContactLinkRequest {
    pub contact_id: i64,
    pub link_type: ContactLinkType,
    pub url: String,
    pub label: Option<String>,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ContactLinksResponse {
    pub links: Vec<ContactLink>,
}
