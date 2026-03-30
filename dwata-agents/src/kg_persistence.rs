use async_trait::async_trait;

use crate::entity_types::ExtractedEntitiesParams;

/// Trait that allows the KG extraction agent to persist entities between passes
/// without depending on `dwata-api`. The concrete implementation lives in
/// `dwata-api::database::kg_entities::KgPersistenceLayer`.
#[async_trait]
pub trait KgPersistenceProvider: Send + Sync {
    /// Persist all entities in `params` that belong to the current pass.
    /// Returns the LLM-ID → DB-ID mapping so subsequent passes can reference
    /// already-persisted entities via negative IDs in the pre-population list.
    async fn persist_pass_result(
        &self,
        params: &ExtractedEntitiesParams,
        source_email_id: Option<i64>,
    ) -> anyhow::Result<()>;
}
