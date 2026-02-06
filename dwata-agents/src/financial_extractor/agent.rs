use crate::storage::{AgentStorage, Message};
use crate::tools::DwataToolExecutor;
use crate::financial_extractor::types::{SavePatternParams, TestPatternParams};
use nocodo_llm_sdk::Tool;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use shared_types::FinancialPattern;
use std::sync::Arc;

pub struct FinancialExtractorAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    tool_executor: Arc<DwataToolExecutor>,
    model: String,
    email_subject: String,
    email_body: String,
    existing_patterns: Vec<FinancialPattern>,
}

impl FinancialExtractorAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        tool_executor: Arc<DwataToolExecutor>,
        model: String,
        email_subject: String,
        email_body: String,
        existing_patterns: Vec<FinancialPattern>,
    ) -> Self {
        Self {
            llm_client,
            storage,
            tool_executor,
            model,
            email_subject,
            email_body,
            existing_patterns,
        }
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<String> {
        let high_signal_line = find_high_signal_line(&self.email_body);

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

        'attempts: for attempt in 0..2 {
            let system_prompt = super::system_prompt::build_system_prompt(
                &self.email_subject,
                &self.email_body,
                &self.existing_patterns,
                high_signal_line.as_deref(),
                attempt == 1,
            );

            let mut saw_tool_call = false;
            let mut test_calls = 0usize;
            let mut successful_test = false;

            for iteration in 0..4 {
                tracing::info!("Agent attempt {} iteration {}", attempt + 1, iteration + 1);

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
                    model: self.model.clone(),
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
                    saw_tool_call = true;
                    let mut retry_attempt = false;

                    for tool_call in tool_calls {
                        match tool_call.name().as_ref() {
                            "test_pattern" => {
                                test_calls += 1;
                                if test_calls > 1 {
                                    return Err(anyhow::anyhow!(
                                        "Multiple test_pattern calls in a single attempt"
                                    ));
                                }

                                let params: TestPatternParams = tool_call.parse_arguments()?;
                                let transactions = self.tool_executor.test_pattern(params).await?;
                                if transactions.is_empty() {
                                    self.storage
                                        .create_message(Message {
                                            id: None,
                                            session_id,
                                            role: "user".to_string(),
                                            content: "Tool result: []".to_string(),
                                        })
                                        .await?;

                                    if attempt == 0 {
                                        self.storage
                                            .create_message(Message {
                                                id: None,
                                                session_id,
                                                role: "user".to_string(),
                                                content: "First attempt failed (no matches). Retry using the high-signal line and a single-line regex anchored on vendor, amount, and date.".to_string(),
                                            })
                                            .await?;
                                        retry_attempt = true;
                                        break;
                                    }

                                    return Err(anyhow::anyhow!(
                                        "No matches found after second attempt"
                                    ));
                                }

                                let tool_result = serde_json::to_string(&transactions)?;
                                self.storage
                                    .create_message(Message {
                                        id: None,
                                        session_id,
                                        role: "user".to_string(),
                                        content: format!("Tool result: {}", tool_result),
                                    })
                                    .await?;

                                successful_test = true;
                                let save_hint = format!(
                                    "Test succeeded. Now call save_pattern using the same regex and group indices. Suggested defaults: name=cerebras_receipt, description=Receipt from vendor with amount and paid date, document_type=receipt, status=paid, confidence=0.9."
                                );
                                self.storage
                                    .create_message(Message {
                                        id: None,
                                        session_id,
                                        role: "user".to_string(),
                                        content: save_hint,
                                    })
                                    .await?;
                            }
                            "save_pattern" => {
                                let params: SavePatternParams = tool_call.parse_arguments()?;
                                let pattern_id = self.tool_executor.save_pattern(params).await?;
                                self.storage
                                    .create_message(Message {
                                        id: None,
                                        session_id,
                                        role: "user".to_string(),
                                        content: format!(
                                            "Tool result: Pattern saved with ID: {}",
                                            pattern_id
                                        ),
                                    })
                                    .await?;
                                return Ok(format!("Pattern saved with ID: {}", pattern_id));
                            }
                            _ => {
                                self.storage
                                    .create_message(Message {
                                        id: None,
                                        session_id,
                                        role: "user".to_string(),
                                        content: format!("Tool result: Unknown tool: {}", tool_call.name()),
                                    })
                                    .await?;
                            }
                        }
                    }

                    if retry_attempt {
                        continue 'attempts;
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

            if successful_test {
                return Err(anyhow::anyhow!(
                    "Test succeeded but save_pattern was not called"
                ));
            }

            if !saw_tool_call && attempt == 0 {
                self
                    .storage
                    .create_message(Message {
                        id: None,
                        session_id,
                        role: "user".to_string(),
                        content: "First attempt failed (no tool calls). Retry using the high-signal line and a single-line regex anchored on vendor, amount, and date.".to_string(),
                    })
                    .await?;
                continue;
            }
        }

        Err(anyhow::anyhow!("Agent failed after two attempts"))
    }
}

fn find_high_signal_line(body: &str) -> Option<String> {
    let re = regex::Regex::new(
        r"(?i)(Receipt from[^\n\r]{0,200}\$[0-9,]+\.\d{2}[^\n\r]{0,200}Paid[^\n\r]{0,200})",
    )
    .ok()?;
    re.captures(body)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
}
