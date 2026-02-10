use crate::storage::{AgentStorage, Message};
use crate::tools::DwataToolExecutor;
use crate::financial_extractor::types::{SavePatternParams, TestPatternParams};
use nocodo_llm_sdk::Tool;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use shared_types::FinancialPattern;
use std::hash::Hasher;
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
            let (system_prompt, email_content_for_user) = super::system_prompt::build_system_prompt(
                &self.email_subject,
                &self.email_body,
                &self.existing_patterns,
                high_signal_line.as_deref(),
                attempt == 1,
                // Pass provider name to optimize for Ollama
                self.llm_client.provider_name(),
            );

            // Create initial message with email content if needed (for Ollama)
            if attempt == 0 {
                let initial_message = if let Some(email_content) = email_content_for_user.as_ref() {
                    format!("Please analyze this email and create a regex pattern to extract the financial data.\n\n{}", email_content)
                } else {
                    "Please analyze this email and create a regex pattern to extract the financial data.".to_string()
                };
                self.storage
                    .create_message(Message {
                        id: None,
                        session_id,
                        role: "user".to_string(),
                        content: initial_message,
                    })
                    .await?;
            }

            let mut saw_tool_call = false;
            let mut test_calls = 0usize;
            let mut successful_test = false;
            let mut saw_save_call = false;
            let mut last_test_params: Option<TestPatternParams> = None;

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
                                if test_calls > 5 {
                                    return Err(anyhow::anyhow!(
                                        "Too many test_pattern calls in a single attempt"
                                    ));
                                }

                                let params: TestPatternParams = tool_call.parse_arguments()?;
                                let transactions = self.tool_executor.test_pattern(params.clone()).await?;
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
                                last_test_params = Some(params);
                                let save_hint = format!(
                                    "Test succeeded. Now call save_pattern using the same regex and group indices. Suggested defaults: name=cerebras_receipt, document_type=receipt, status=paid."
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
                                saw_save_call = true;
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

                    if successful_test && !saw_save_call {
                        if let Some(params) = last_test_params {
                            let fallback_name = generate_fallback_name(&params.regex_pattern);
                            let auto_params = SavePatternParams {
                                name: fallback_name,
                                regex_pattern: params.regex_pattern,
                                document_type: "receipt".to_string(),
                                status: "paid".to_string(),
                                amount_group: params.amount_group,
                                source_vendor_group: params.source_vendor_group,
                                destination_vendor_group: params.destination_vendor_group,
                                date_group: params.date_group,
                                reference_group: params.reference_group,
                            };
                            let pattern_id = self.tool_executor.save_pattern(auto_params).await?;
                            return Ok(format!("Pattern saved with ID: {}", pattern_id));
                        }
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

fn generate_fallback_name(regex_pattern: &str) -> String {
    let mut slug = String::new();
    let mut last_was_underscore = false;

    for ch in regex_pattern.chars() {
        let is_alnum = ch.is_ascii_alphanumeric();
        if is_alnum {
            slug.push(ch.to_ascii_lowercase());
            last_was_underscore = false;
        } else if !last_was_underscore {
            slug.push('_');
            last_was_underscore = true;
        }
    }

    let slug = slug.trim_matches('_');
    let mut slug = if slug.is_empty() {
        "pattern".to_string()
    } else {
        slug.to_string()
    };
    if slug.len() > 32 {
        slug.truncate(32);
    }

    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&regex_pattern, &mut hasher);
    std::hash::Hash::hash(&now, &mut hasher);
    let hash = hasher.finish();
    let suffix = to_base36(hash % 36_u64.pow(4));

    if suffix.is_empty() {
        slug
    } else {
        format!("{slug}_{suffix}")
    }
}

fn to_base36(mut value: u64) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let rem = (value % 36) as u8;
        let ch = if rem < 10 {
            (b'0' + rem) as char
        } else {
            (b'a' + (rem - 10)) as char
        };
        out.push(ch);
        value /= 36;
    }
    out.iter().rev().collect()
}
