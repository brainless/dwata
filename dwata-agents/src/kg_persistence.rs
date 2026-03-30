use async_trait::async_trait;

use crate::entity_types::ExtractedEntitiesParams;

/// Trait that allows the KG extraction agent to persist entities between passes
/// without depending on `dwata-api`. The concrete implementation lives in
/// `dwata-api::database::kg_entities::KgPersistenceLayer`.
#[async_trait]
pub trait KgPersistenceProvider: Send + Sync {
    /// Persist all entities in `params` that belong to the current pass.
    ///
    /// `sender_email` is the From address of the source email. Implementations
    /// may use it to backfill the email field on organisations/persons that the
    /// LLM identified as the sender but left without an email address.
    async fn persist_pass_result(
        &self,
        params: &ExtractedEntitiesParams,
        source_email_id: Option<i64>,
        sender_email: Option<&str>,
    ) -> anyhow::Result<()>;
}
