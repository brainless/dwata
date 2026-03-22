use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for the `search_emails` tool — what the LLM sees.
/// `from_address` is always fixed to the current email's sender internally;
/// `keywords` is mandatory to keep queries focused.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchEmailsParams {
    /// Keywords to search for in the subject or body of previous emails from the same sender.
    pub keywords: String,

    /// Maximum number of emails to return. Defaults to 5 if not specified. Maximum is 10.
    pub limit: Option<u8>,
}

/// A single email result returned to the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSearchResult {
    pub subject: String,
    pub from: String,
    pub date: Option<String>,
    /// Plain-text excerpt of the cleaned email body.
    pub body_excerpt: String,
}

/// Abstraction over the search backend. Implemented in `dwata-api` using Tantivy.
/// `from_address` is baked into the implementation at construction time.
#[async_trait]
pub trait EmailSearchProvider: Send + Sync {
    async fn search_emails(
        &self,
        params: &SearchEmailsParams,
    ) -> anyhow::Result<Vec<EmailSearchResult>>;
}
