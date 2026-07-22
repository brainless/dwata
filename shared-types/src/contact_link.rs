use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContactLinkType {
    Linkedin,
    Github,
    Twitter,
    Personal,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactLink {
    pub id: i64,
    /// FK to the persons table.
    pub person_id: i64,
    pub link_type: ContactLinkType,
    pub url: String,
    pub label: Option<String>,
    pub is_primary: bool,
    pub is_verified: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ContactLinksResponse {
    pub links: Vec<ContactLink>,
}
