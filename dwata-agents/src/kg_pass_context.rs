use std::sync::Arc;

use crate::entity_search::{
    EntitySearchProvider, EntitySearchResult, NamedEntityKind, SearchEntitiesParams,
};
use crate::entity_type_manifest::{existing_entities_section, generate_entity_manifest};

// Re-export KgPassType from shared_types
pub use shared_types::KgPassType;

/// Extension trait for KgPassType with agent-specific methods
pub trait KgPassTypeExt {
    fn entity_types(&self) -> Vec<NamedEntityKind>;
    fn search_types(&self) -> Vec<NamedEntityKind>;
}

impl KgPassTypeExt for KgPassType {
    fn entity_types(&self) -> Vec<NamedEntityKind> {
        match self {
            KgPassType::IdentityResolution => vec![
                NamedEntityKind::Location,
                NamedEntityKind::Organisation,
                NamedEntityKind::Person,
            ],
            KgPassType::FinancialExtraction => vec![
                NamedEntityKind::Bill,
                NamedEntityKind::Transaction,
                NamedEntityKind::Subscription,
            ],
            KgPassType::EventExtraction => vec![NamedEntityKind::Event],
            KgPassType::OrderExtraction => vec![NamedEntityKind::Order],
        }
    }

    fn search_types(&self) -> Vec<NamedEntityKind> {
        match self {
            KgPassType::IdentityResolution => vec![
                NamedEntityKind::Organisation,
                NamedEntityKind::Person,
                NamedEntityKind::Location,
            ],
            KgPassType::FinancialExtraction => {
                vec![NamedEntityKind::Organisation, NamedEntityKind::Subscription]
            }
            KgPassType::EventExtraction => vec![NamedEntityKind::Location, NamedEntityKind::Person],
            KgPassType::OrderExtraction => vec![NamedEntityKind::Organisation],
        }
    }
}

#[derive(Debug, Clone)]
pub struct KgExtractionPass {
    pub pass_type: KgPassType,
    pub existing_entities: Vec<EntitySearchResult>,
    pub source_content: String,
    pub sender_email: Option<String>,
}

impl KgExtractionPass {
    pub fn new(pass_type: KgPassType, source_content: String) -> Self {
        Self {
            pass_type,
            existing_entities: Vec::new(),
            source_content,
            sender_email: None,
        }
    }

    pub fn with_sender_email(mut self, email: String) -> Self {
        self.sender_email = Some(email);
        self
    }

    /// Populate `existing_entities` by running all pre-population steps against
    /// the search provider. Each step is independent — results are merged and
    /// deduplicated by entity ID. Add new steps here as the extractor improves.
    pub async fn populate_existing_entities(
        mut self,
        search_provider: Option<&Arc<dyn EntitySearchProvider>>,
    ) -> Self {
        let Some(provider) = search_provider else {
            return self;
        };

        let mut results: Vec<EntitySearchResult> = Vec::new();

        // --- Pre-population step 1: BM25 keyword search ---
        // Extract keywords from the Subject line and search the Tantivy entity
        // index. Keeps the query focused on entity-naming terms (org/person names)
        // rather than body boilerplate.
        {
            let keywords = extract_subject_keywords(&self.source_content);
            if !keywords.is_empty() {
                let params = SearchEntitiesParams {
                    keywords,
                    entity_types: self.pass_type.search_types(),
                    limit: Some(5),
                    sender_email: None,
                };
                match provider.search_entities(&params).await {
                    Ok(r) => results.extend(r),
                    Err(e) => tracing::warn!("Pre-population step 1 (BM25) failed: {}", e),
                }
            }
        }

        // --- Pre-population step 2: Direct sender email lookup ---
        // Look up organisations and persons by exact sender email/domain. More
        // reliable than BM25 for identity resolution when the sender is already
        // in the KG (e.g. a recurring billing org).
        if let Some(ref email) = self.sender_email {
            let types = self.pass_type.search_types();
            let needs_lookup = types.contains(&NamedEntityKind::Organisation)
                || types.contains(&NamedEntityKind::Person);

            if needs_lookup {
                let params = SearchEntitiesParams {
                    keywords: String::new(),
                    entity_types: types,
                    limit: Some(5),
                    sender_email: Some(email.clone()),
                };
                match provider.search_entities(&params).await {
                    Ok(sender_results) => {
                        let seen: std::collections::HashSet<i64> =
                            results.iter().map(|r| r.id).collect();
                        results
                            .extend(sender_results.into_iter().filter(|r| !seen.contains(&r.id)));
                    }
                    Err(e) => {
                        tracing::warn!("Pre-population step 2 (sender email) failed: {}", e)
                    }
                }
            }
        }

        self.existing_entities = results;
        self
    }

    pub fn build_system_prompt(&self) -> String {
        use crate::kg_pass_context::KgPassTypeExt;

        let manifest = generate_entity_manifest(Some(&self.pass_type.entity_types()));
        let existing = existing_entities_section(&self.existing_entities);
        let pass_desc = self.pass_type.description();
        let pass_name = self.pass_type.name();

        // For identity resolution, add sender-specific rules.
        let sender_instruction = if matches!(self.pass_type, KgPassType::IdentityResolution) {
            let mut parts = Vec::new();

            if let Some(ref email) = self.sender_email {
                parts.push(format!(
                    "\n7. The sender is identified by the From line (address: **{}**). \
                         For any organisation or person you extract as the sender, \
                         their email field is MANDATORY — do not leave it blank.",
                    email
                ));
            }

            parts.push(
                "\n8. For every person you extract, set `organisation_id` to the `id` of \
                     the organisation they belong to — including organisations extracted in \
                     this same pass. For example: if you extract org id=1 (Linode) and a \
                     person whose email is billing@linode.com, set that person's \
                     `organisation_id` to 1."
                    .to_string(),
            );

            parts.join("")
        } else {
            String::new()
        };

        format!(
            "You are a knowledge graph extraction agent running the **{}** pass.\n\n\
            {}\n\n\
            ## Current Pass: {}\n\n\
            {}\n\n\
            ## Source Content\n\n\
            {}\n\n\
            ## Instructions\n\n\
            1. Extract all {} entities from the source content above.\n\
            2. Link them to existing entities where appropriate using FK id references.\n\
            3. Assign each new entity a unique positive integer `id` (you choose).\n\
            4. Copy raw date/amount strings exactly as they appear — do not reformat.\n\
            5. For `search_summary`: write a 1-2 sentence BM25-searchable summary capturing relational context (e.g. 'streaming service, monthly billing via credit card').\n\
            6. Call `submit_entities` once you have extracted all entities for this pass.{}",
            pass_name,
            manifest,
            pass_desc,
            if existing.is_empty() {
                String::new()
            } else {
                format!("{}\n", existing)
            },
            self.source_content,
            self.pass_type.name().replace('_', " "),
            sender_instruction,
        )
    }
}

/// Extract BM25 search keywords from the Subject line of the email content.
///
/// Using only the subject keeps the query focused on entity-naming terms
/// (org names, person names) rather than body boilerplate like "Company",
/// "Payment", "Number", or receipt reference IDs.
fn extract_subject_keywords(content: &str) -> String {
    let subject = content
        .lines()
        .find(|l| l.starts_with("Subject: "))
        .and_then(|l| l.strip_prefix("Subject: "))
        .unwrap_or("");

    let mut seen = std::collections::HashSet::new();
    subject
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .filter(|w| seen.insert(*w))
        .take(10)
        .collect::<Vec<_>>()
        .join(" ")
}
