use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Type of named entity in the knowledge graph
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, ts_rs::TS,
)]
#[ts(export)]
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

/// Parameters for searching entities
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ts_rs::TS)]
#[ts(export)]
pub struct SearchEntitiesParams {
    pub keywords: String,
    pub entity_types: Vec<NamedEntityKind>,
    pub limit: Option<u8>,
    /// If set, also look up organisations/persons by exact email address or domain.
    /// Used by the sender-email pre-population step.
    pub sender_email: Option<String>,
}

/// Result of an entity search
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct EntitySearchResult {
    pub id: i64,
    pub entity_type: NamedEntityKind,
    pub score: f32,
    pub name: String,
    pub search_summary: Option<String>,
}
