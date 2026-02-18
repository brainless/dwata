use crate::storage::{AgentStorage, Message};
use crate::template_bill_extractor::types::TranslateBillVariablesParams;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;
use std::sync::Arc;

pub struct TemplateBillExtractorAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    model: String,
    template: String,
}

impl TemplateBillExtractorAgent {
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

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<TranslateBillVariablesParams> {
        let system_prompt = super::prompts::build_system_prompt(&self.model, &self.template);

        let translate_tool = Tool::from_type::<TranslateBillVariablesParams>()
            .name("translate_bill_variables")
            .description(
                "Map each template placeholder to a bill field name. Use exact field names: \
                 total-amount, currency, issued-date, due-date, billing-period-start, \
                 billing-period-end, document-reference, service-identifier. Set field to null \
                 if the placeholder does not map to any bill field.",
            )
            .build();

        let tools = vec![translate_tool];

        self.storage
            .create_message(Message {
                id: None,
                session_id,
                role: "user".to_string(),
                content: "Map the bill placeholders in the template to bill field names."
                    .to_string(),
            })
            .await?;

        for iteration in 0..3 {
            tracing::info!("Bill extractor iteration {}", iteration + 1);

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
                system: Some(system_prompt.clone()),
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
                    if tool_call.name() == "translate_bill_variables" {
                        let params: TranslateBillVariablesParams = tool_call.parse_arguments()?;

                        self.storage
                            .create_message(Message {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                content: format!(
                                    "Tool result: bill translations accepted: {} mappings",
                                    params.translations.len()
                                ),
                            })
                            .await?;

                        return Ok(params);
                    }
                }
            }

            self.storage
                .create_message(Message {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    content: "Please call the `translate_bill_variables` tool with your mappings."
                        .to_string(),
                })
                .await?;
        }

        Err(anyhow::anyhow!(
            "Bill extractor did not call translate_bill_variables after 3 iterations"
        ))
    }
}
