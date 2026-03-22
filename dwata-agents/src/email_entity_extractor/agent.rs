use crate::email_entity_extractor::search::{EmailSearchProvider, SearchEmailsParams};
use crate::email_entity_extractor::types::{
    parse_value, ConfirmEntitiesParams, ExtractedEntitiesParams,
};
use crate::storage::{AgentStorage, Message};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage};
use nocodo_llm_sdk::Tool;
use std::sync::Arc;

const MAX_SUBMIT_ITERATIONS: usize = 3;

pub struct EmailEntityExtractorAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    model: String,
    email_subject: String,
    email_body: String,
    search_provider: Option<Arc<dyn EmailSearchProvider>>,
}

impl EmailEntityExtractorAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        model: String,
        email_subject: String,
        email_body: String,
        search_provider: Option<Arc<dyn EmailSearchProvider>>,
    ) -> Self {
        Self {
            llm_client,
            storage,
            model,
            email_subject,
            email_body,
            search_provider,
        }
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<ExtractedEntitiesParams> {
        let system_prompt =
            super::prompts::build_system_prompt(&self.email_subject, &self.email_body);

        let submit_tool = Tool::from_type::<ExtractedEntitiesParams>()
            .name("submit_entities")
            .description("Submit all entities extracted from the email.")
            .build();

        let confirm_tool = Tool::from_type::<ConfirmEntitiesParams>()
            .name("confirm_entities")
            .description(
                "Confirm that the parsed entity values shown to you are correct, \
                 or reject them so you can revise and resubmit.",
            )
            .build();

        let search_tool = Tool::from_type::<SearchEmailsParams>()
            .name("search_emails")
            .description(
                "Search previous emails from the same sender to gather extra context. \
                 Use this when the current email references a prior order, subscription, \
                 or relationship that needs clarification.",
            )
            .build();

        let mut tools = vec![submit_tool, confirm_tool];
        if self.search_provider.is_some() {
            tools.push(search_tool);
        }

        self.storage
            .create_message(Message {
                id: None,
                session_id,
                role: "user".to_string(),
                content: "Extract all entities from this email now.".to_string(),
            })
            .await?;

        let mut last_params: Option<ExtractedEntitiesParams> = None;
        let mut submit_count = 0;

        loop {
            tracing::info!(
                "Email entity extractor iteration {} (submit_count={})",
                submit_count + 1,
                submit_count
            );

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
                temperature: Some(0.1),
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
                for tool_call in &tool_calls {
                    tracing::info!(
                        tool = tool_call.name(),
                        arguments = %tool_call.arguments(),
                        "Model called tool"
                    );
                    if tool_call.name() == "search_emails" {
                        let params: SearchEmailsParams = tool_call.parse_arguments()?;
                        let result_text = match &self.search_provider {
                            Some(provider) => match provider.search_emails(&params).await {
                                Ok(results) if results.is_empty() => {
                                    "No previous emails from this sender matched your query."
                                        .to_string()
                                }
                                Ok(results) => {
                                    let mut text = format!(
                                        "Found {} previous email(s) from this sender:\n\n",
                                        results.len()
                                    );
                                    for (i, r) in results.iter().enumerate() {
                                        text.push_str(&format!(
                                                "--- Email {} ---\nSubject: {}\nFrom: {}\nDate: {}\n\n{}\n\n",
                                                i + 1,
                                                r.subject,
                                                r.from,
                                                r.date.as_deref().unwrap_or("unknown"),
                                                r.body_excerpt,
                                            ));
                                    }
                                    text
                                }
                                Err(e) => {
                                    tracing::warn!("search_emails tool failed: {}", e);
                                    "Email search failed; proceed with available information."
                                        .to_string()
                                }
                            },
                            None => "Email search is not available.".to_string(),
                        };

                        self.storage
                            .create_message(Message {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                content: result_text,
                            })
                            .await?;
                        break;
                    } else if tool_call.name() == "submit_entities" {
                        let params: ExtractedEntitiesParams = tool_call.parse_arguments()?;
                        submit_count += 1;
                        let table = build_parsed_table(&params);
                        last_params = Some(params);

                        if submit_count >= MAX_SUBMIT_ITERATIONS {
                            tracing::info!(
                                "Reached max submit iterations ({}), returning last extracted entities",
                                MAX_SUBMIT_ITERATIONS
                            );
                            return last_params
                                .ok_or_else(|| anyhow::anyhow!("No entities were extracted"));
                        }

                        let confirmation_msg = super::prompts::build_confirmation_message(&table);
                        self.storage
                            .create_message(Message {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                content: confirmation_msg,
                            })
                            .await?;
                        break;
                    } else if tool_call.name() == "confirm_entities" {
                        let confirm: ConfirmEntitiesParams = tool_call.parse_arguments()?;
                        if confirm.confirmed {
                            tracing::info!("Model confirmed entities");
                            return last_params.ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Model confirmed but no entities were submitted yet"
                                )
                            });
                        } else {
                            let note = confirm
                                .note
                                .unwrap_or_else(|| "no reason given".to_string());
                            tracing::info!("Model rejected entities: {}", note);
                            self.storage
                                .create_message(Message {
                                    id: None,
                                    session_id,
                                    role: "user".to_string(),
                                    content: format!(
                                        "Understood. Please call `submit_entities` again with the corrected entities. (Your note: {})",
                                        note
                                    ),
                                })
                                .await?;
                            break;
                        }
                    }
                }
            } else {
                self.storage
                    .create_message(Message {
                        id: None,
                        session_id,
                        role: "user".to_string(),
                        content: "Please call `submit_entities` with all entities you found."
                            .to_string(),
                    })
                    .await?;
            }

            if submit_count >= MAX_SUBMIT_ITERATIONS {
                break;
            }
        }

        last_params.ok_or_else(|| anyhow::anyhow!("Email entity extractor: no entities returned"))
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers (used for the confirmation message sent back to the LLM)
// ---------------------------------------------------------------------------

fn build_parsed_table(params: &ExtractedEntitiesParams) -> String {
    let mut out = String::new();

    if let Some(locations) = &params.locations {
        if !locations.is_empty() {
            out.push_str("### Locations\n");
            out.push_str(&row(
                "id",
                "city",
                "country_code",
                "address_line1",
                "postal_code",
            ));
            out.push_str(&sep());
            for l in locations {
                out.push_str(&row(
                    &l.id.to_string(),
                    opt(&l.city),
                    opt(&l.country_code),
                    opt(&l.address_line1),
                    opt(&l.postal_code),
                ));
            }
            out.push('\n');
        }
    }

    if let Some(organisations) = &params.organisations {
        if !organisations.is_empty() {
            out.push_str("### Organisations\n");
            out.push_str(&row("id", "name", "role", "website", "location_id"));
            out.push_str(&sep());
            for o in organisations {
                let role_str = o.role.map(|r| r.to_string());
                out.push_str(&row(
                    &o.id.to_string(),
                    &o.name,
                    opt(&role_str),
                    opt(&o.website),
                    &o.location_id.map(|v| v.to_string()).unwrap_or_default(),
                ));
            }
            out.push('\n');
        }
    }

    if let Some(persons) = &params.persons {
        if !persons.is_empty() {
            out.push_str("### Persons\n");
            out.push_str(&row("id", "name", "email", "phone", "organisation_id"));
            out.push_str(&sep());
            for p in persons {
                out.push_str(&row(
                    &p.id.to_string(),
                    &p.name,
                    opt(&p.email),
                    opt(&p.phone),
                    &p.organisation_id.map(|v| v.to_string()).unwrap_or_default(),
                ));
            }
            out.push('\n');
        }
    }

    if let Some(bills) = &params.bills {
        if !bills.is_empty() {
            out.push_str("### Bills\n");
            out.push_str(&row(
                "id",
                "doc_type",
                "total_amount (parsed)",
                "currency",
                "issued_date (parsed)",
            ));
            out.push_str(&sep());
            for b in bills {
                let amount_parsed = b
                    .total_amount
                    .as_deref()
                    .map(|v| parse_value(v).to_string())
                    .unwrap_or_default();
                let date_parsed = b
                    .issued_date
                    .as_deref()
                    .map(|v| parse_value(v).to_string())
                    .unwrap_or_default();
                out.push_str(&row(
                    &b.id.to_string(),
                    opt(&b.document_reference),
                    &amount_parsed,
                    opt(&b.currency),
                    &date_parsed,
                ));
            }
            out.push('\n');
        }
    }

    if let Some(transactions) = &params.transactions {
        if !transactions.is_empty() {
            out.push_str("### Transactions\n");
            out.push_str(&row(
                "id",
                "amount (parsed)",
                "currency",
                "date (parsed)",
                "reference",
            ));
            out.push_str(&sep());
            for t in transactions {
                let amount_parsed = format!("{:.2}", t.amount);
                let date_parsed = t
                    .transaction_date
                    .as_deref()
                    .map(|v| parse_value(v).to_string())
                    .unwrap_or_default();
                out.push_str(&row(
                    &t.id.to_string(),
                    &amount_parsed,
                    &t.currency,
                    &date_parsed,
                    opt(&t.transaction_reference),
                ));
            }
            out.push('\n');
        }
    }

    if let Some(subscriptions) = &params.subscriptions {
        if !subscriptions.is_empty() {
            out.push_str("### Subscriptions\n");
            out.push_str(&row(
                "id",
                "service_name",
                "plan_name",
                "billing_cycle",
                "amount (parsed)",
            ));
            out.push_str(&sep());
            for s in subscriptions {
                let amount_parsed = s.amount.map(|v| format!("{:.2}", v)).unwrap_or_default();
                out.push_str(&row(
                    &s.id.to_string(),
                    &s.service_name,
                    opt(&s.plan_name),
                    opt(&s.billing_cycle),
                    &amount_parsed,
                ));
            }
            out.push('\n');
        }
    }

    if let Some(orders) = &params.orders {
        if !orders.is_empty() {
            out.push_str("### Orders\n");
            out.push_str(&row(
                "id",
                "order_reference",
                "status",
                "total_amount (parsed)",
                "tracking_number",
            ));
            out.push_str(&sep());
            for o in orders {
                let amount_parsed = o
                    .total_amount
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_default();
                out.push_str(&row(
                    &o.id.to_string(),
                    opt(&o.order_reference),
                    opt(&o.status),
                    &amount_parsed,
                    opt(&o.tracking_number),
                ));
            }
            out.push('\n');
        }
    }

    if let Some(events) = &params.events {
        if !events.is_empty() {
            out.push_str("### Events\n");
            out.push_str(&row(
                "id",
                "name",
                "event_date (parsed)",
                "attendees",
                "location_id",
            ));
            out.push_str(&sep());
            for e in events {
                let date_parsed = e
                    .event_date
                    .as_deref()
                    .map(|v| parse_value(v).to_string())
                    .unwrap_or_default();
                let attendees = e.attendees.as_deref().unwrap_or(&[]).join(", ");
                out.push_str(&row(
                    &e.id.to_string(),
                    &e.name,
                    &date_parsed,
                    &attendees,
                    &e.location_id.map(|v| v.to_string()).unwrap_or_default(),
                ));
            }
            out.push('\n');
        }
    }

    if out.is_empty() {
        out.push_str("(no entities extracted)\n");
    }

    out
}

fn opt(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("")
}

fn row(a: &str, b: &str, c: &str, d: &str, e: &str) -> String {
    format!(
        "| {:<20} | {:<30} | {:<30} | {:<40} | {:<20} |\n",
        truncate(a, 20),
        truncate(b, 30),
        truncate(c, 30),
        truncate(d, 40),
        truncate(e, 20)
    )
}

fn sep() -> String {
    format!(
        "| {:-<20} | {:-<30} | {:-<30} | {:-<40} | {:-<20} |\n",
        "", "", "", "", ""
    )
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
