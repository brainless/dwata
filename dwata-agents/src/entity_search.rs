use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NamedEntityKind {
    Location,
    Organisation,
    Person,
    Bill,
    Transaction,
    Subscription,
    Order,
    Event,
}

impl NamedEntityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NamedEntityKind::Location => "location",
            NamedEntityKind::Organisation => "organisation",
            NamedEntityKind::Person => "person",
            NamedEntityKind::Bill => "bill",
            NamedEntityKind::Transaction => "transaction",
            NamedEntityKind::Subscription => "subscription",
            NamedEntityKind::Order => "order",
            NamedEntityKind::Event => "event",
        }
    }

    pub fn plural(&self) -> &'static str {
        match self {
            NamedEntityKind::Location => "locations",
            NamedEntityKind::Organisation => "organisations",
            NamedEntityKind::Person => "persons",
            NamedEntityKind::Bill => "bills",
            NamedEntityKind::Transaction => "transactions",
            NamedEntityKind::Subscription => "subscriptions",
            NamedEntityKind::Order => "orders",
            NamedEntityKind::Event => "events",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchEntitiesParams {
    pub keywords: String,
    pub entity_types: Vec<NamedEntityKind>,
    pub limit: Option<u8>,
    /// If set, also look up organisations/persons by exact email address or domain.
    /// Used by the sender-email pre-population step.
    pub sender_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySearchResult {
    pub id: i64,
    pub entity_type: NamedEntityKind,
    pub score: f32,
    pub name: String,
    pub search_summary: Option<String>,
}

#[async_trait]
pub trait EntitySearchProvider: Send + Sync {
    async fn search_entities(
        &self,
        params: &SearchEntitiesParams,
    ) -> anyhow::Result<Vec<EntitySearchResult>>;
}
