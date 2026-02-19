use crate::storage::{AgentStorage, Message};
use crate::template_financial_extractor::types::{TransactionField, TranslateVariablesParams};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;
use std::sync::Arc;

pub struct TemplateFinancialExtractorAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    model: String,
    template: String,
}

impl TemplateFinancialExtractorAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        model: String,
        template: String,
    ) -> Self {
        Self {
            llm_client,
            storage,
            model,
            template,
        }
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<TranslateVariablesParams> {
        let system_prompt = super::prompts::get_system_prompt(&self.model, &self.template);

        let translate_tool = Tool::from_type::<TranslateVariablesParams>()
            .name("translate_variables")
            .description(
                "Map each template placeholder to a transaction field name. Use exact field \
                 names: amount, currency, transaction-date, vendor, transaction-reference. \
                 Set field to null if the placeholder does not map to any transaction field. \
                 For a transaction template, amount is mandatory and must be mapped at least once.",
            )
            .build();

        let tools = vec![translate_tool];

        // Initial user message
        self.storage
            .create_message(Message {
                id: None,
                session_id,
                role: "user".to_string(),
                content: "Please analyze the template and translate the placeholder variables to financial field names.".to_string(),
            })
            .await?;

        for iteration in 0..3 {
            tracing::info!("Template extractor iteration {}", iteration + 1);

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
                max_tokens: 4096,
                model: self.model.clone(),
                system: Some(system_prompt.clone()),
                // GPT-5 nano/mini don't support custom temperature or top_p
                temperature: if self.model.contains("nano") || self.model.contains("mini") {
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

            // Store assistant response
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
                for tool_call in tool_calls {
                    if tool_call.name() == "translate_variables" {
                        let params: TranslateVariablesParams = tool_call.parse_arguments()?;
                        let has_amount = params
                            .translations
                            .iter()
                            .any(|t| t.field == Some(TransactionField::Amount));

                        if !has_amount {
                            self.storage
                                .create_message(Message {
                                    id: None,
                                    session_id,
                                    role: "user".to_string(),
                                    content: "Invalid mapping: transaction templates must include at least one placeholder mapped to `amount`. Re-check placeholders and call `translate_variables` again.".to_string(),
                                })
                                .await?;
                            continue;
                        }

                        // Store the tool result
                        self.storage
                            .create_message(Message {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                content: format!(
                                    "Tool result: translations accepted: {:?}",
                                    params.translations
                                ),
                            })
                            .await?;

                        return Ok(params);
                    }
                }
            }

            // No tool call — ask again
            self.storage
                .create_message(Message {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    content: "Please call the translate_variables tool with your mappings."
                        .to_string(),
                })
                .await?;
        }

        Err(anyhow::anyhow!(
            "Agent did not call translate_variables after 3 iterations"
        ))
    }
}
