use std::sync::Arc;

use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;

use crate::entity_search::EntitySearchProvider;
use crate::entity_types::{ConfirmEntitiesParams, ExtractedEntitiesParams};
use crate::extraction_state::{
    count_entities_by_type, ExtractionStateProvider, ExtractionStep, RetryReason,
};
use crate::kg_email_extractor::types::LabelDocumentParams;
use crate::kg_pass_context::{KgExtractionPass, KgPassType};
use crate::kg_persistence::KgPersistenceProvider;
use crate::storage::{AgentStorage, Message};

const MAX_TOOL_WAIT_ITERATIONS: usize = 3;
const MAX_PARSE_RETRIES_PER_PASS: usize = 3;
const MAX_EMPTY_CONFIRM_RETRIES_PER_PASS: usize = 1;
const MAX_CONFIRM_BEFORE_SUBMIT_RETRIES_PER_PASS: usize = 3;

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
    state_provider: Option<Arc<dyn ExtractionStateProvider>>,
    model: String,
    email_content: String,
    label: Option<LabelDocumentParams>,
    source_email_id: Option<i64>,
    sender_name: Option<String>,
    sender_email: Option<String>,
    single_tool_submission: bool,
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
            state_provider: None,
            model,
            email_content,
            label: None,
            source_email_id: None,
            sender_name: None,
            sender_email: None,
            single_tool_submission: false,
        }
    }

    pub fn with_search_provider(mut self, provider: Arc<dyn EntitySearchProvider>) -> Self {
        self.search_provider = Some(provider);
        self
    }

    pub fn with_extraction_state(mut self, provider: Arc<dyn ExtractionStateProvider>) -> Self {
        self.state_provider = Some(provider);
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

    pub fn with_sender(mut self, name: Option<String>, email: Option<String>) -> Self {
        self.sender_name = name;
        self.sender_email = email;
        self
    }

    /// When enabled, the agent uses only `submit_entities` and persists
    /// immediately after a valid submit without requiring `confirm_entities`.
    pub fn with_single_tool_submission(mut self, enabled: bool) -> Self {
        self.single_tool_submission = enabled;
        self
    }

    /// Build the full content string sent to the LLM: prepends a `From:` line
    /// when sender info is available so the model sees who sent the email.
    fn build_content(&self) -> String {
        let from_line = match (&self.sender_name, &self.sender_email) {
            (Some(name), Some(email)) => format!("From: {} <{}>\n", name, email),
            (None, Some(email)) => format!("From: {}\n", email),
            (Some(name), None) => format!("From: {}\n", name),
            (None, None) => String::new(),
        };
        format!("{}{}", from_line, self.email_content)
    }

    /// Initialize extraction state if a state provider is configured
    async fn initialize_state(&self, session_id: i64) {
        if let Some(ref provider) = self.state_provider {
            provider
                .initialize_state(session_id, self.source_email_id, self.sender_email.clone())
                .await;
        }
    }

    /// Record a step event if a state provider is configured
    async fn record_step(&self, session_id: i64, step: ExtractionStep) {
        if let Some(ref provider) = self.state_provider {
            provider.record_step(session_id, step).await;
        }
    }

    /// Complete extraction state
    async fn complete_extraction(&self, session_id: i64) {
        if let Some(ref provider) = self.state_provider {
            provider.complete_extraction(session_id).await;
        }
    }

    /// Mark extraction as failed
    async fn fail_extraction(&self, session_id: i64, error_message: String) {
        if let Some(ref provider) = self.state_provider {
            provider.fail_extraction(session_id, error_message).await;
        }
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<()> {
        // Initialize extraction state
        self.initialize_state(session_id).await;

        let passes = self.active_passes();
        let content = self.build_content();
        tracing::info!(
            model = %self.model,
            passes = passes.len(),
            content_len = content.len(),
            "KG extraction starting"
        );

        for (i, pass_type) in passes.iter().enumerate() {
            tracing::info!("Starting KG pass: {:?}", pass_type);

            // Record pass started
            self.record_step(
                session_id,
                ExtractionStep::PassStarted {
                    timestamp: current_timestamp(),
                    pass_type: *pass_type,
                    pass_name: pass_type.name().to_string(),
                },
            )
            .await;

            let mut pass = KgExtractionPass::new(*pass_type, content.clone());
            if let Some(ref email) = self.sender_email {
                pass = pass.with_sender_email(email.clone());
            }
            let pass = pass
                .populate_existing_entities(self.search_provider.as_ref())
                .await;

            // Record search steps from the pass
            self.record_search_steps(session_id, *pass_type, &pass)
                .await;

            let system_prompt = pass.build_system_prompt();
            tracing::debug!(
                pass = %pass_type.name(),
                prompt_len = system_prompt.len(),
                "KG pass prompt built"
            );
            let expect_non_empty = self.expect_non_empty(pass_type);
            let mut start_msg = super::prompts::start_pass_message(pass_type.name());
            if expect_non_empty {
                start_msg.push_str(
                    " The document label says financial data exists for this email. \
                     Do not return an empty payload unless the source clearly has no bill or transaction fields.",
                );
            }
            let pass_history_start_index = self.storage.get_messages(session_id).await?.len();

            self.storage
                .create_message(Message {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    content: start_msg,
                })
                .await?;

            match self
                .run_pass_loop(
                    session_id,
                    *pass_type,
                    pass_type.name(),
                    &system_prompt,
                    expect_non_empty,
                    pass_history_start_index,
                    self.single_tool_submission,
                )
                .await
            {
                Ok(entities) => {
                    tracing::info!(
                        "Persisting entities from pass {:?} (source_email_id={:?})",
                        pass_type,
                        self.source_email_id
                    );

                    // Record entities extracted
                    let (entity_counts, total_entities) = count_entities_by_type(&entities);
                    self.record_step(
                        session_id,
                        ExtractionStep::EntitiesExtracted {
                            timestamp: current_timestamp(),
                            pass_type: *pass_type,
                            entities: entities.clone(),
                            entity_counts,
                            total_entities,
                        },
                    )
                    .await;

                    self.persistence
                        .persist_pass_result(
                            &entities,
                            self.source_email_id,
                            self.sender_email.as_deref(),
                        )
                        .await?;

                    // Record pass completed
                    self.record_step(
                        session_id,
                        ExtractionStep::PassCompleted {
                            timestamp: current_timestamp(),
                            pass_type: *pass_type,
                            entities_persisted: true,
                        },
                    )
                    .await;

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
                Err(e) => {
                    // Record pass failed
                    self.record_step(
                        session_id,
                        ExtractionStep::PassFailed {
                            timestamp: current_timestamp(),
                            pass_type: *pass_type,
                            error_message: e.to_string(),
                        },
                    )
                    .await;

                    self.fail_extraction(session_id, e.to_string()).await;
                    return Err(e);
                }
            }
        }

        // Complete extraction
        self.complete_extraction(session_id).await;

        Ok(())
    }

    /// Record search steps from a completed pass
    async fn record_search_steps(
        &self,
        session_id: i64,
        pass_type: KgPassType,
        pass: &KgExtractionPass,
    ) {
        // Record the combined search results that were found
        if !pass.existing_entities.is_empty() {
            // Determine which type of search(es) produced these results
            // For now, we record them as a combined search step
            let entity_types: Vec<String> = pass_type
                .search_types()
                .iter()
                .map(|t| t.as_str().to_string())
                .collect();

            self.record_step(
                session_id,
                ExtractionStep::SearchPerformed {
                    timestamp: current_timestamp(),
                    pass_type,
                    keywords: extract_subject_keywords(&pass.source_content),
                    entity_types,
                    results: pass.existing_entities.clone(),
                    result_count: pass.existing_entities.len(),
                },
            )
            .await;
        }
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

    fn expect_non_empty(&self, pass_type: &KgPassType) -> bool {
        match pass_type {
            KgPassType::FinancialExtraction => self
                .label
                .as_ref()
                .map(|l| l.has_bill || l.has_transaction)
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Drive one pass: loop until the LLM calls `submit_entities` (with optional
    /// `confirm_entities` confirmation). Returns the final extracted payload.
    async fn run_pass_loop(
        &self,
        session_id: i64,
        pass_type: KgPassType,
        pass_name: &str,
        system_prompt: &str,
        expect_non_empty: bool,
        history_start_index: usize,
        single_tool_submission: bool,
    ) -> anyhow::Result<ExtractedEntitiesParams> {
        let submit_tool = Tool::from_type::<ExtractedEntitiesParams>()
            .name("submit_entities")
            .description("Submit all entities extracted from the email for this pass.")
            .build();

        let tools = if single_tool_submission {
            vec![submit_tool]
        } else {
            let confirm_tool = Tool::from_type::<ConfirmEntitiesParams>()
                .name("confirm_entities")
                .description(
                    "Confirm that the parsed entity values shown to you are correct, \
                     or reject them so you can revise and resubmit.",
                )
                .build();
            vec![submit_tool, confirm_tool]
        };
        let mut last_entities: Option<ExtractedEntitiesParams> = None;
        let mut nudge_count = 0;
        let mut parse_retry_count = 0;
        let mut empty_confirm_retry_count = 0;
        let mut confirm_before_submit_retry_count = 0;
        let mut iteration = 0;
        let initial_messages = self.storage.get_messages(session_id).await?;
        let mut llm_messages: Vec<LlmMessage> = initial_messages
            .iter()
            .skip(history_start_index)
            .map(|msg| match msg.role.as_str() {
                "assistant" => LlmMessage::assistant(&msg.content),
                "system" => LlmMessage::system(&msg.content),
                "tool" => LlmMessage::tool("stored_tool_message", &msg.content),
                _ => LlmMessage::user(&msg.content),
            })
            .collect();

        loop {
            iteration += 1;

            let request = CompletionRequest {
                messages: llm_messages.clone(),
                max_tokens: 2048,
                model: self.model.clone(),
                system: Some(system_prompt.to_string()),
                temperature: if self.model.contains("nano")
                    || self.model.contains("mini")
                    || self.model.contains("qwen")
                {
                    None
                } else {
                    Some(0.1)
                },
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: None,
                response_format: None,
            };

            let response = self.llm_client.complete(request).await?;
            tracing::debug!(
                pass = %pass_name,
                model = %self.model,
                prompt_len = system_prompt.len(),
                content_blocks = response.content.len(),
                tool_calls = response.tool_calls.as_ref().map(|t| t.len()).unwrap_or(0),
                "KG pass response received"
            );

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
            let assistant_text = response
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            llm_messages.push(LlmMessage::assistant(assistant_text));

            if let Some(tool_calls) = response.tool_calls {
                let mut handled_tool = false;
                for tool_call in &tool_calls {
                    tracing::info!(tool = tool_call.name(), "KG pass: model called tool");

                    // Record tool call
                    self.record_step(
                        session_id,
                        ExtractionStep::ToolCallMade {
                            timestamp: current_timestamp(),
                            pass_type,
                            tool_name: tool_call.name().to_string(),
                            iteration,
                        },
                    )
                    .await;

                    if tool_call.name() == "submit_entities" {
                        let entities: ExtractedEntitiesParams = match tool_call.parse_arguments() {
                            Ok(v) => v,
                            Err(e) => {
                                parse_retry_count += 1;
                                tracing::warn!(
                                    pass = %pass_name,
                                    retry = parse_retry_count,
                                    max_retries = MAX_PARSE_RETRIES_PER_PASS,
                                    raw_arguments = %tool_call.raw_arguments(),
                                    "submit_entities parse failed: {}",
                                    e
                                );

                                // Record retry
                                self.record_step(
                                    session_id,
                                    ExtractionStep::RetryOccurred {
                                        timestamp: current_timestamp(),
                                        pass_type,
                                        reason: RetryReason::ParseFailed,
                                        attempt: parse_retry_count,
                                        max_attempts: MAX_PARSE_RETRIES_PER_PASS,
                                    },
                                )
                                .await;

                                if parse_retry_count >= MAX_PARSE_RETRIES_PER_PASS {
                                    return Err(anyhow::anyhow!(
                                        "submit_entities parse failed after {} retries: {}",
                                        MAX_PARSE_RETRIES_PER_PASS,
                                        e
                                    ));
                                }

                                self.storage
                                    .create_message(Message {
                                        id: None,
                                        session_id,
                                        role: "user".to_string(),
                                        content: format!(
                                            "The `submit_entities` payload could not be parsed: {}. \
                                             Resubmit with the exact tool schema. Every entity requires `id`.",
                                            e
                                        ),
                                    })
                                    .await?;
                                llm_messages.push(LlmMessage::user(format!(
                                    "The `submit_entities` payload could not be parsed: {}. \
                                     Resubmit with the exact tool schema. Every entity requires `id`.",
                                    e
                                )));
                                handled_tool = true;
                                break;
                            }
                        };
                        last_entities = Some(entities);
                        handled_tool = true;

                        if single_tool_submission {
                            return Ok(last_entities.expect("entities just assigned"));
                        }

                        let tool_result = "Entities received. Call `confirm_entities` with \
                                           confirmed=true to persist, or confirmed=false to revise."
                            .to_string();
                        self.storage
                            .create_message(Message {
                                id: None,
                                session_id,
                                role: "tool".to_string(),
                                content: tool_result.clone(),
                            })
                            .await?;
                        llm_messages.push(LlmMessage::tool(tool_call.id(), tool_result));
                        break;
                    } else if !single_tool_submission && tool_call.name() == "confirm_entities" {
                        let confirm: ConfirmEntitiesParams = match tool_call.parse_arguments() {
                            Ok(v) => v,
                            Err(e) => {
                                parse_retry_count += 1;
                                tracing::warn!(
                                    pass = %pass_name,
                                    retry = parse_retry_count,
                                    max_retries = MAX_PARSE_RETRIES_PER_PASS,
                                    raw_arguments = %tool_call.raw_arguments(),
                                    "confirm_entities parse failed: {}",
                                    e
                                );

                                // Record retry
                                self.record_step(
                                    session_id,
                                    ExtractionStep::RetryOccurred {
                                        timestamp: current_timestamp(),
                                        pass_type,
                                        reason: RetryReason::ParseFailed,
                                        attempt: parse_retry_count,
                                        max_attempts: MAX_PARSE_RETRIES_PER_PASS,
                                    },
                                )
                                .await;

                                if parse_retry_count >= MAX_PARSE_RETRIES_PER_PASS {
                                    return Err(anyhow::anyhow!(
                                        "confirm_entities parse failed after {} retries: {}",
                                        MAX_PARSE_RETRIES_PER_PASS,
                                        e
                                    ));
                                }
                                self.storage
                                    .create_message(Message {
                                        id: None,
                                        session_id,
                                        role: "user".to_string(),
                                        content: format!(
                                            "The `confirm_entities` payload could not be parsed: {}. \
                                             Call `confirm_entities` again with valid JSON.",
                                            e
                                        ),
                                    })
                                    .await?;
                                llm_messages.push(LlmMessage::user(format!(
                                    "The `confirm_entities` payload could not be parsed: {}. \
                                     Call `confirm_entities` again with valid JSON.",
                                    e
                                )));
                                handled_tool = true;
                                break;
                            }
                        };
                        handled_tool = true;

                        if last_entities.is_none() {
                            confirm_before_submit_retry_count += 1;
                            tracing::warn!(
                                pass = %pass_name,
                                retry = confirm_before_submit_retry_count,
                                max_retries = MAX_CONFIRM_BEFORE_SUBMIT_RETRIES_PER_PASS,
                                "Model called confirm_entities before submit_entities"
                            );

                            // Record retry
                            self.record_step(
                                session_id,
                                ExtractionStep::RetryOccurred {
                                    timestamp: current_timestamp(),
                                    pass_type,
                                    reason: RetryReason::ConfirmBeforeSubmit,
                                    attempt: confirm_before_submit_retry_count,
                                    max_attempts: MAX_CONFIRM_BEFORE_SUBMIT_RETRIES_PER_PASS,
                                },
                            )
                            .await;

                            if confirm_before_submit_retry_count
                                >= MAX_CONFIRM_BEFORE_SUBMIT_RETRIES_PER_PASS
                            {
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
                                    role: "tool".to_string(),
                                    content: "You must call `submit_entities` before `confirm_entities`. \
                                              Submit entities now (or an explicit empty payload) and then confirm."
                                        .to_string(),
                                })
                                .await?;
                            llm_messages.push(LlmMessage::tool(
                                tool_call.id(),
                                "You must call `submit_entities` before `confirm_entities`. \
                                 Submit entities now (or an explicit empty payload) and then confirm."
                                    .to_string(),
                            ));
                            break;
                        }

                        if confirm.confirmed {
                            let entities = last_entities.clone().ok_or_else(|| {
                                anyhow::anyhow!("Model confirmed but no entities were submitted")
                            })?;

                            if expect_non_empty && entities_is_empty(&entities) {
                                empty_confirm_retry_count += 1;
                                tracing::warn!(
                                    pass = %pass_name,
                                    retry = empty_confirm_retry_count,
                                    max_retries = MAX_EMPTY_CONFIRM_RETRIES_PER_PASS,
                                    "Model confirmed empty entities despite expected extraction"
                                );

                                // Record retry
                                self.record_step(
                                    session_id,
                                    ExtractionStep::RetryOccurred {
                                        timestamp: current_timestamp(),
                                        pass_type,
                                        reason: RetryReason::EmptyConfirm,
                                        attempt: empty_confirm_retry_count,
                                        max_attempts: MAX_EMPTY_CONFIRM_RETRIES_PER_PASS,
                                    },
                                )
                                .await;

                                if empty_confirm_retry_count > MAX_EMPTY_CONFIRM_RETRIES_PER_PASS {
                                    return Ok(entities);
                                }
                                self.storage
                                    .create_message(Message {
                                        id: None,
                                        session_id,
                                        role: "tool".to_string(),
                                        content: "The email was labeled as containing bill/transaction data, \
                                                  but your submitted payload is empty. \
                                                  Re-check the email and call `submit_entities` again. \
                                                  Include at least the obvious financial entities when present."
                                            .to_string(),
                                    })
                                    .await?;
                                llm_messages.push(LlmMessage::tool(
                                    tool_call.id(),
                                    "The email was labeled as containing bill/transaction data, \
                                     but your submitted payload is empty. \
                                     Re-check the email and call `submit_entities` again. \
                                     Include at least the obvious financial entities when present."
                                        .to_string(),
                                ));
                                break;
                            }

                            return Ok(entities);
                        } else {
                            let note = confirm
                                .note
                                .unwrap_or_else(|| "no reason given".to_string());
                            tracing::info!("Model rejected entities: {}", note);
                            self.storage
                                .create_message(Message {
                                    id: None,
                                    session_id,
                                    role: "tool".to_string(),
                                    content: format!(
                                        "Understood. Call `submit_entities` again with corrections. (Note: {})",
                                        note
                                    ),
                                })
                                .await?;
                            llm_messages.push(LlmMessage::tool(
                                tool_call.id(),
                                format!(
                                    "Understood. Call `submit_entities` again with corrections. (Note: {})",
                                    note
                                ),
                            ));
                            break;
                        }
                    }
                }
                if !handled_tool {
                    self.storage
                        .create_message(Message {
                            id: None,
                            session_id,
                            role: "user".to_string(),
                            content: super::prompts::nudge_message().to_string(),
                        })
                        .await?;
                    llm_messages.push(LlmMessage::user(super::prompts::nudge_message()));
                }
            } else {
                nudge_count += 1;
                tracing::warn!(
                    nudge = nudge_count,
                    max_nudges = MAX_TOOL_WAIT_ITERATIONS,
                    "KG pass response had no tool calls"
                );

                // Record retry
                self.record_step(
                    session_id,
                    ExtractionStep::RetryOccurred {
                        timestamp: current_timestamp(),
                        pass_type,
                        reason: RetryReason::NoToolCalls,
                        attempt: nudge_count,
                        max_attempts: MAX_TOOL_WAIT_ITERATIONS,
                    },
                )
                .await;

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
                llm_messages.push(LlmMessage::user(super::prompts::nudge_message()));
            }
        }
    }
}

fn entities_is_empty(entities: &ExtractedEntitiesParams) -> bool {
    entities
        .locations
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
        && entities
            .organisations
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        && entities
            .persons
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        && entities
            .bills
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        && entities
            .transactions
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        && entities
            .subscriptions
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        && entities
            .orders
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        && entities
            .events
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
}

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Extract BM25 search keywords from the Subject line of the email content.
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
