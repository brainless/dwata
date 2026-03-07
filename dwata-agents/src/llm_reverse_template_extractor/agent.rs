use crate::llm_reverse_template_extractor::types::{ReverseTemplateParams, ReverseTemplateType};
use crate::storage::{AgentStorage, Message};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;
use std::sync::Arc;

pub struct LlmReverseTemplateExtractorAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    model: String,
    template_type: ReverseTemplateType,
    sample_subject: String,
    sample_body: String,
}

impl LlmReverseTemplateExtractorAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        template_type: ReverseTemplateType,
        sample_subject: String,
        sample_body: String,
    ) -> Self {
        Self {
            llm_client,
            storage,
            model: MINISTRAL_3_3B_ID.to_string(),
            template_type,
            sample_subject,
            sample_body,
        }
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<ReverseTemplateParams> {
        let system_prompt = super::prompts::build_system_prompt(
            self.template_type,
            &self.sample_subject,
            &self.sample_body,
        );

        let reverse_tool = Tool::from_type::<ReverseTemplateParams>()
            .name("submit_reverse_template")
            .description("Submit the reconstructed email source template.")
            .build();

        self.storage
            .create_message(Message {
                id: None,
                session_id,
                role: "user".to_string(),
                content: "Generate the reconstructed source template for this sample email now."
                    .to_string(),
            })
            .await?;

        for iteration in 0..2 {
            tracing::info!("Reverse template extractor iteration {}", iteration + 1);

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
                max_tokens: 1024,
                model: self.model.clone(),
                system: Some(system_prompt.clone()),
                temperature: Some(0.1),
                top_p: None,
                stop_sequences: None,
                tools: Some(vec![reverse_tool.clone()]),
                tool_choice: None,
                response_format: None,
            };

            let response = match self.llm_client.complete(request).await {
                Ok(resp) => resp,
                Err(err) => {
                    let err_msg = err.to_string();
                    if err_msg.contains("invalid character '\\n' in string literal")
                        && iteration == 0
                    {
                        self.storage
                            .create_message(Message {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                content: "Tool-call JSON was invalid. Call `submit_reverse_template` again, but ensure `template_body` is a valid JSON string with escaped newlines (`\\n`) and no raw newline characters inside the JSON string."
                                    .to_string(),
                            })
                            .await?;
                        continue;
                    }
                    return Err(err.into());
                }
            };

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
                    if tool_call.name() == "submit_reverse_template" {
                        let params: ReverseTemplateParams = tool_call.parse_arguments()?;
                        if !params.template_body.contains("Subject:") {
                            self.storage
                                .create_message(Message {
                                    id: None,
                                    session_id,
                                    role: "user".to_string(),
                                    content:
                                        "Invalid output: template must include a `Subject:` line."
                                            .to_string(),
                                })
                                .await?;
                            continue;
                        }
                        if !params.template_body.contains("---") {
                            self.storage
                                .create_message(Message {
                                    id: None,
                                    session_id,
                                    role: "user".to_string(),
                                    content: "Invalid output: template must include `---` between subject and body."
                                        .to_string(),
                                })
                                .await?;
                            continue;
                        }
                        return Ok(params);
                    }
                }
            }

            self.storage
                .create_message(Message {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    content: "Please call `submit_reverse_template`.".to_string(),
                })
                .await?;
        }

        Err(anyhow::anyhow!(
            "Reverse extractor did not call submit_reverse_template"
        ))
    }
}
