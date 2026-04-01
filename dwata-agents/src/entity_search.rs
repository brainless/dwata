use async_trait::async_trait;

// Re-export search types from shared_types
pub use shared_types::{EntitySearchResult, NamedEntityKind, SearchEntitiesParams};

#[async_trait]
pub trait EntitySearchProvider: Send + Sync {
    async fn search_entities(
        &self,
        params: &SearchEntitiesParams,
    ) -> anyhow::Result<Vec<EntitySearchResult>>;
}
