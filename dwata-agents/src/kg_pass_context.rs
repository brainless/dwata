use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::entity_search::{
    EntitySearchProvider, EntitySearchResult, NamedEntityKind, SearchEntitiesParams,
};
use crate::entity_type_manifest::{existing_entities_section, generate_entity_manifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum KgPassType {
    IdentityResolution,
    FinancialExtraction,
    EventExtraction,
    OrderExtraction,
}

impl KgPassType {
    pub fn name(&self) -> &'static str {
        match self {
            KgPassType::IdentityResolution => "identity_resolution",
            KgPassType::FinancialExtraction => "financial_extraction",
            KgPassType::EventExtraction => "event_extraction",
            KgPassType::OrderExtraction => "order_extraction",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            KgPassType::IdentityResolution => {
                "Extract locations, organisations, and persons with their relationships"
            }
            KgPassType::FinancialExtraction => {
                "Extract bills, transactions, and subscriptions linked to identified entities"
            }
            KgPassType::EventExtraction => "Extract calendar events and meetings",
            KgPassType::OrderExtraction => "Extract e-commerce orders and shipments",
        }
    }

    pub fn entity_types(&self) -> Vec<NamedEntityKind> {
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

    pub fn search_types(&self) -> Vec<NamedEntityKind> {
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
}

impl KgExtractionPass {
    pub fn new(pass_type: KgPassType, source_content: String) -> Self {
        Self {
            pass_type,
            existing_entities: Vec::new(),
            source_content,
        }
    }

    pub async fn populate_existing_entities(
        mut self,
        search_provider: Option<&Arc<dyn EntitySearchProvider>>,
    ) -> Self {
        if let Some(provider) = search_provider {
            let search_types = self.pass_type.search_types();
            let keywords = extract_keywords(&self.source_content);

            let params = SearchEntitiesParams {
                keywords,
                entity_types: search_types,
                limit: Some(5),
            };

            match provider.search_entities(&params).await {
                Ok(results) => {
                    self.existing_entities = results;
                }
                Err(e) => {
                    tracing::warn!("Failed to search existing entities: {}", e);
                }
            }
        }
        self
    }

    pub fn build_system_prompt(&self) -> String {
        let manifest = generate_entity_manifest(Some(&self.pass_type.entity_types()));
        let existing = existing_entities_section(&self.existing_entities);
        let pass_desc = self.pass_type.description();
        let pass_name = self.pass_type.name();

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
            6. Call `submit_entities` once you have extracted all entities for this pass.",
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
        )
    }
}

fn extract_keywords(content: &str) -> String {
    let words: Vec<&str> = content
        .split(|c: char| !c.is_alphanumeric() && c != ' ')
        .filter(|w| w.len() > 2)
        .collect();

    let unique: Vec<&str> = {
        let mut seen = std::collections::HashSet::new();
        words.into_iter().filter(|w| seen.insert(*w)).collect()
    };

    unique
        .iter()
        .take(10)
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}
