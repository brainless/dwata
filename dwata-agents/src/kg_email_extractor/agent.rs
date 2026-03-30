use std::sync::Arc;

use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;

use crate::entity_search::EntitySearchProvider;
use crate::entity_types::{ConfirmEntitiesParams, ExtractedEntitiesParams};
use crate::kg_email_extractor::types::LabelDocumentParams;
use crate::kg_pass_context::{KgExtractionPass, KgPassType};
use crate::kg_persistence::KgPersistenceProvider;
use crate::storage::{AgentStorage, Message};

const MAX_TOOL_WAIT_ITERATIONS: usize = 3;

/// Runs four sequential KG extraction passes against a single email:
///
/// 1. **IdentityResolution** — always runs; extracts locations, organisations, persons.
/// 2. **FinancialExtraction** — gated on `label.has_bill || label.has_transaction`.
/// 3. **EventExtraction** — gated on `label.has_event`.
/// 4. **OrderExtraction** — gated on `label.has_order`.
///
/// After each pass the persistence provider writes entities to the DB and the
/// entity search index, so the next pass can find them via pre-population.
pub struct KgEmailExtractionAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    persistence: Arc<dyn KgPersistenceProvider>,
    search_provider: Option<Arc<dyn EntitySearchProvider>>,
    model: String,
    email_content: String,
    label: Option<LabelDocumentParams>,
    source_email_id: Option<i64>,
}

impl KgEmailExtractionAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        persistence: Arc<dyn KgPersistenceProvider>,
        model: String,
        email_content: String,
    ) -> Self {
        Self {
            llm_client,
            storage,
            persistence,
            search_provider: None,
            model,
            email_content,
            label: None,
            source_email_id: None,
        }
    }

    pub fn with_search_provider(mut self, provider: Arc<dyn EntitySearchProvider>) -> Self {
        self.search_provider = Some(provider);
        self
    }

    pub fn with_label(mut self, label: LabelDocumentParams) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_source_email_id(mut self, id: i64) -> Self {
        self.source_email_id = Some(id);
        self
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<()> {
        let passes = self.active_passes();

        for (i, pass_type) in passes.iter().enumerate() {
            tracing::info!("Starting KG pass: {:?}", pass_type);

            let pass = KgExtractionPass::new(*pass_type, self.email_content.clone())
                .populate_existing_entities(self.search_provider.as_ref())
                .await;

            let system_prompt = pass.build_system_prompt();
            let start_msg = super::prompts::start_pass_message(pass_type.name());

            self.storage
                .create_message(Message {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    content: start_msg,
                })
                .await?;

            let entities = self.run_pass_loop(session_id, &system_prompt).await?;

            tracing::info!(
                "Persisting entities from pass {:?} (source_email_id={:?})",
                pass_type,
                self.source_email_id
            );

            self.persistence
                .persist_pass_result(&entities, self.source_email_id)
                .await?;

            // Notify the LLM that the pass is done and the next one is starting,
            // so it keeps context across passes.
            if i + 1 < passes.len() {
                let next = passes[i + 1];
                let transition =
                    super::prompts::pass_complete_message(pass_type.name(), next.name());
                self.storage
                    .create_message(Message {
                        id: None,
                        session_id,
                        role: "user".to_string(),
                        content: transition,
                    })
                    .await?;
            }
        }

        Ok(())
    }

    // -------------------------------------------------------------------------

    fn active_passes(&self) -> Vec<KgPassType> {
        let mut passes = vec![KgPassType::IdentityResolution];

        if let Some(ref label) = self.label {
            if label.has_bill || label.has_transaction {
                passes.push(KgPassType::FinancialExtraction);
            }
            if label.has_event {
                passes.push(KgPassType::EventExtraction);
            }
            if label.has_order {
                passes.push(KgPassType::OrderExtraction);
            }
        } else {
            // No label available — run all passes conservatively.
            passes.push(KgPassType::FinancialExtraction);
            passes.push(KgPassType::EventExtraction);
            passes.push(KgPassType::OrderExtraction);
        }

        passes
    }

    /// Drive one pass: loop until the LLM calls `submit_entities` (with optional
    /// `confirm_entities` confirmation). Returns the final extracted payload.
    async fn run_pass_loop(
        &self,
        session_id: i64,
        system_prompt: &str,
    ) -> anyhow::Result<ExtractedEntitiesParams> {
        let submit_tool = Tool::from_type::<ExtractedEntitiesParams>()
            .name("submit_entities")
            .description("Submit all entities extracted from the email for this pass.")
            .build();

        let confirm_tool = Tool::from_type::<ConfirmEntitiesParams>()
            .name("confirm_entities")
            .description(
                "Confirm that the parsed entity values shown to you are correct, \
                 or reject them so you can revise and resubmit.",
            )
            .build();

        let tools = vec![submit_tool, confirm_tool];
        let mut last_entities: Option<ExtractedEntitiesParams> = None;
        let mut nudge_count = 0;

        loop {
            let messages = self.storage.get_messages(session_id).await?;
            let llm_messages: Vec<LlmMessage> = messages
                .iter()
                .map(|msg| {
                    if msg.role == "user" {
                        LlmMessage::user(&msg.content)
                    } else {
                        LlmMessage::assistant(&msg.content)
                    }
                })
                .collect();

            let request = CompletionRequest {
                messages: llm_messages,
                max_tokens: 2048,
                model: self.model.clone(),
                system: Some(system_prompt.to_string()),
                temperature: Some(0.1),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: None,
                response_format: None,
            };

            let response = self.llm_client.complete(request).await?;

            self.storage
                .create_message(Message {
                    id: None,
                    session_id,
                    role: "assistant".to_string(),
                    content: response
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                })
                .await?;

            if let Some(tool_calls) = response.tool_calls {
                for tool_call in &tool_calls {
                    tracing::info!(tool = tool_call.name(), "KG pass: model called tool");

                    if tool_call.name() == "submit_entities" {
                        let entities: ExtractedEntitiesParams = tool_call.parse_arguments()?;
                        last_entities = Some(entities);

                        // Ask the model to confirm before we persist.
                        self.storage
                            .create_message(Message {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                content: "Entities received. Call `confirm_entities` with \
                                          confirmed=true to persist, or confirmed=false to revise."
                                    .to_string(),
                            })
                            .await?;
                        break;
                    } else if tool_call.name() == "confirm_entities" {
                        let confirm: ConfirmEntitiesParams = tool_call.parse_arguments()?;
                        if confirm.confirmed {
                            return last_entities.ok_or_else(|| {
                                anyhow::anyhow!("Model confirmed but no entities were submitted")
                            });
                        } else {
                            let note = confirm
                                .note
                                .unwrap_or_else(|| "no reason given".to_string());
                            tracing::info!("Model rejected entities: {}", note);
                            self.storage
                                .create_message(Message {
                                    id: None,
                                    session_id,
                                    role: "user".to_string(),
                                    content: format!(
                                        "Understood. Call `submit_entities` again with corrections. (Note: {})",
                                        note
                                    ),
                                })
                                .await?;
                            break;
                        }
                    }
                }
            } else {
                nudge_count += 1;
                if nudge_count >= MAX_TOOL_WAIT_ITERATIONS {
                    // If we have a partial result from a previous submit, use it.
                    if let Some(entities) = last_entities {
                        tracing::warn!(
                            "LLM did not confirm after {} nudges; using last submitted entities",
                            MAX_TOOL_WAIT_ITERATIONS
                        );
                        return Ok(entities);
                    }
                    // Otherwise return empty to avoid blocking the pipeline.
                    tracing::warn!(
                        "LLM did not call submit_entities after {} nudges; returning empty",
                        MAX_TOOL_WAIT_ITERATIONS
                    );
                    return Ok(ExtractedEntitiesParams {
                        locations: None,
                        organisations: None,
                        persons: None,
                        bills: None,
                        transactions: None,
                        subscriptions: None,
                        orders: None,
                        events: None,
                    });
                }

                self.storage
                    .create_message(Message {
                        id: None,
                        session_id,
                        role: "user".to_string(),
                        content: super::prompts::nudge_message().to_string(),
                    })
                    .await?;
            }
        }
    }
}
