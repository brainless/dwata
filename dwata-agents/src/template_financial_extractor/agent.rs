use crate::storage::{AgentStorage, Message};
use crate::template_financial_extractor::types::TranslateVariablesParams;
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
                "Translate generic placeholder names from the email template to financial field \
                 template strings. Each key is a placeholder name (e.g. 'placeholder_1') and \
                 the value is a Jinja2 template string using financial field names (amount, \
                 currency, transaction_date, category, vendor), or null if the placeholder \
                 does not map to a financial field.",
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
