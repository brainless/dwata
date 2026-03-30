use crate::kg_email_extractor::types::{DocumentType, LabelDocumentParams};
use crate::storage::{AgentStorage, Message};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;
use std::sync::Arc;

const RESPONSE_PREVIEW_CHARS: usize = 1200;

fn preview_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn fallback_label_from_template(template: &str) -> LabelDocumentParams {
    let text = template.to_ascii_lowercase();

    let has_receipt = text.contains("receipt");
    let has_payment = text.contains("payment")
        || text.contains("paid")
        || text.contains("debited")
        || text.contains("charged");
    let has_bill = text.contains("amount due")
        || text.contains("due date")
        || text.contains("pay by")
        || text.contains("billing period")
        || text.contains("invoice");
    let has_order = text.contains("order")
        || text.contains("shipment")
        || text.contains("tracking")
        || text.contains("delivered");
    let has_event = text.contains("meeting")
        || text.contains("appointment")
        || text.contains("invite")
        || text.contains("calendar")
        || text.contains("event");

    let doc_type = if has_receipt {
        DocumentType::Receipt
    } else if has_bill {
        DocumentType::Bill
    } else if has_payment {
        DocumentType::PaymentConfirmation
    } else if has_order {
        DocumentType::Unknown
    } else if has_event {
        DocumentType::Unknown
    } else {
        DocumentType::Unknown
    };

    LabelDocumentParams {
        doc_type,
        has_bill,
        has_transaction: has_payment || has_receipt,
        has_event,
        has_order,
    }
}

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
        let system_prompt = super::document_labeler_prompt::build_system_prompt(&self.template);
        tracing::info!(
            model = %self.model,
            prompt_len = system_prompt.len(),
            template_len = self.template.len(),
            "Document labeler starting"
        );

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
            let assistant_text = response
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            tracing::debug!(
                model = %self.model,
                iteration = iteration + 1,
                content_blocks = response.content.len(),
                tool_calls = response.tool_calls.as_ref().map(|t| t.len()).unwrap_or(0),
                assistant_text_preview = %preview_text(&assistant_text, RESPONSE_PREVIEW_CHARS),
                "Document labeler response received"
            );

            self.storage
                .create_message(Message {
                    id: None,
                    session_id,
                    role: "assistant".to_string(),
                    content: assistant_text.clone(),
                })
                .await?;

            if let Some(tool_calls) = response.tool_calls {
                for tool_call in tool_calls {
                    tracing::debug!(
                        iteration = iteration + 1,
                        tool_id = %tool_call.id(),
                        tool_name = %tool_call.name(),
                        raw_arguments = %tool_call.raw_arguments(),
                        "Document labeler tool call"
                    );
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
                tracing::warn!(
                    iteration = iteration + 1,
                    "Document labeler returned tool calls but none matched label_document"
                );
            } else {
                tracing::warn!(
                    iteration = iteration + 1,
                    assistant_text_preview = %preview_text(&assistant_text, RESPONSE_PREVIEW_CHARS),
                    "Document labeler returned no tool calls"
                );
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

        let fallback = fallback_label_from_template(&self.template);
        tracing::warn!(
            model = %self.model,
            fallback_doc_type = ?fallback.doc_type,
            fallback_has_bill = fallback.has_bill,
            fallback_has_transaction = fallback.has_transaction,
            fallback_has_event = fallback.has_event,
            fallback_has_order = fallback.has_order,
            "Document labeler did not call label_document after 2 iterations; using heuristic fallback"
        );
        Ok(fallback)
    }
}
