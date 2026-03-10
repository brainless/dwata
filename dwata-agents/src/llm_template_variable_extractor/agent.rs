use crate::llm_template_variable_extractor::types::{TemplateVariableParams, TemplateVariableType};
use crate::storage::{AgentStorage, Message};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;
use std::sync::Arc;

pub struct LlmTemplateVariableExtractorAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    model: String,
    template_type: TemplateVariableType,
    sample_subject: String,
    sample_body: String,
}

impl LlmTemplateVariableExtractorAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        model: String,
        template_type: TemplateVariableType,
        sample_subject: String,
        sample_body: String,
    ) -> Self {
        Self {
            llm_client,
            storage,
            model,
            template_type,
            sample_subject,
            sample_body,
        }
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<TemplateVariableParams> {
        let system_prompt = super::prompts::build_system_prompt(
            self.template_type.clone(),
            &self.sample_subject,
            &self.sample_body,
        );

        let variable_tool = Tool::from_type::<TemplateVariableParams>()
            .name("submit_template_variables")
            .description("Submit the extracted template variables with their values.")
            .build();

        self.storage
            .create_message(Message {
                id: None,
                session_id,
                role: "user".to_string(),
                content: "Extract all template variables from this email sample now.".to_string(),
            })
            .await?;

        for iteration in 0..2 {
            tracing::info!("Template variable extractor iteration {}", iteration + 1);

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
                tools: Some(vec![variable_tool.clone()]),
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
                                content: "Tool-call JSON was invalid. Call `submit_template_variables` again, but ensure all string values are valid JSON strings with escaped newlines (`\\n`) and no raw newline characters inside the JSON string."
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
                    if tool_call.name() == "submit_template_variables" {
                        let params: TemplateVariableParams = tool_call.parse_arguments()?;
                        if params.variables.is_empty() {
                            self.storage
                                .create_message(Message {
                                    id: None,
                                    session_id,
                                    role: "user".to_string(),
                                    content:
                                        "Invalid output: at least one variable must be extracted."
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
                    content: "Please call `submit_template_variables`.".to_string(),
                })
                .await?;
        }

        Err(anyhow::anyhow!(
            "Template variable extractor did not call submit_template_variables"
        ))
    }
}
