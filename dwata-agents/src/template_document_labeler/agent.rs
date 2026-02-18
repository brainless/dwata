use crate::storage::{AgentStorage, Message};
use crate::template_document_labeler::types::LabelDocumentParams;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;
use std::sync::Arc;

pub struct TemplateDocumentLabelerAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    model: String,
    template: String,
}

impl TemplateDocumentLabelerAgent {
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

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<LabelDocumentParams> {
        let system_prompt = super::prompts::build_system_prompt(&self.template);

        let label_tool = Tool::from_type::<LabelDocumentParams>()
            .name("label_document")
            .description(
                "Classify the financial document type and determine whether it contains a bill \
                 (amount due / billing period) and/or a completed transaction (payment confirmed).",
            )
            .build();

        let tools = vec![label_tool];

        self.storage
            .create_message(Message {
                id: None,
                session_id,
                role: "user".to_string(),
                content: "Classify this financial document template.".to_string(),
            })
            .await?;

        for iteration in 0..2 {
            tracing::info!("Document labeler iteration {}", iteration + 1);

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
                max_tokens: 512,
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
                    if tool_call.name() == "label_document" {
                        let params: LabelDocumentParams = tool_call.parse_arguments()?;

                        self.storage
                            .create_message(Message {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                content: format!(
                                    "Tool result: label accepted: doc_type={:?}, has_bill={}, has_transaction={}",
                                    params.doc_type, params.has_bill, params.has_transaction
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
                    content: "Please call the `label_document` tool with your classification."
                        .to_string(),
                })
                .await?;
        }

        Err(anyhow::anyhow!(
            "Document labeler did not call label_document after 2 iterations"
        ))
    }
}
