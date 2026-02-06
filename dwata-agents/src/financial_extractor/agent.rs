use crate::storage::{AgentStorage, Message, Session};
use crate::tools::DwataToolExecutor;
use crate::financial_extractor::types::{SavePatternParams, TestPatternParams};
use nocodo_llm_sdk::Tool;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage, Role};
use shared_types::FinancialPattern;
use std::sync::Arc;

pub struct FinancialExtractorAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    tool_executor: Arc<DwataToolExecutor>,
    email_subject: String,
    email_body: String,
    existing_patterns: Vec<FinancialPattern>,
}

impl FinancialExtractorAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        tool_executor: Arc<DwataToolExecutor>,
        email_subject: String,
        email_body: String,
        existing_patterns: Vec<FinancialPattern>,
    ) -> Self {
        Self {
            llm_client,
            storage,
            tool_executor,
            email_subject,
            email_body,
            existing_patterns,
        }
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<String> {
        let system_prompt = super::system_prompt::build_system_prompt(
            &self.email_subject,
            &self.email_body,
            &self.existing_patterns,
        );

        let initial_message = "Please analyze this email and create a regex pattern to extract the financial data.";
        self.storage
            .create_message(Message {
                id: None,
                session_id,
                role: "user".to_string(),
                content: initial_message.to_string(),
            })
            .await?;

        let test_pattern_tool = Tool::from_type::<TestPatternParams>()
            .name("test_pattern")
            .description("Test a regex pattern against the email content")
            .build();

        let save_pattern_tool = Tool::from_type::<SavePatternParams>()
            .name("save_pattern")
            .description("Save a validated pattern to the database")
            .build();

        let tools = vec![test_pattern_tool, save_pattern_tool];

        for iteration in 0..30 {
            tracing::info!("Agent iteration {}", iteration);

            let messages = self.storage.get_messages(session_id).await?;

            let mut llm_messages = Vec::new();

            for msg in &messages {
                if msg.role == "user" {
                    llm_messages.push(LlmMessage::user(&msg.content));
                } else if msg.role == "assistant" {
                    llm_messages.push(LlmMessage::assistant(&msg.content));
                }
            }

            let request = CompletionRequest {
                messages: llm_messages,
                max_tokens: 4096,
                model: "claude-sonnet-4-5-20250929".to_string(),
                system: Some(system_prompt.clone()),
                temperature: None,
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
                    let tool_result = match tool_call.name().as_ref() {
                        "test_pattern" => {
                            let params: TestPatternParams = tool_call.parse_arguments()?;
                            let transactions = self.tool_executor.test_pattern(params).await?;
                            serde_json::to_string(&transactions)?
                        }
                        "save_pattern" => {
                            let params: SavePatternParams = tool_call.parse_arguments()?;
                            let pattern_id = self.tool_executor.save_pattern(params).await?;
                            format!("Pattern saved with ID: {}", pattern_id)
                        }
                        _ => {
                            format!("Unknown tool: {}", tool_call.name())
                        }
                    };

                    self.storage
                        .create_message(Message {
                            id: None,
                            session_id,
                            role: "user".to_string(),
                            content: format!("Tool result: {}", tool_result),
                        })
                        .await?;
                }

                continue;
            }

            return Ok(response
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"));
        }

        Err(anyhow::anyhow!("Agent exceeded maximum iterations"))
    }
}
