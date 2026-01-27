# Implement LLM-Based Extractor

## Objective

Implement LLM-based extraction for complex reasoning and disambiguation. This extractor handles cases where pattern matching and NER models struggle, using language model reasoning to understand context and extract accurate information.

**Confidence Target**: 0.7-0.95 with good prompts and reasoning

## Dependencies

**Blocked by**: Task #1 - Extraction Framework Foundation

Requires the `Extractor` trait and common types from shared-types.

## Background

LLMs excel at understanding context, handling ambiguity, and making nuanced decisions:
- Distinguish between project deadline vs task due date based on context
- Infer task priority from email tone and urgency indicators
- Extract relationships between entities (which task belongs to which project)
- Handle conversational language and implicit information

**Use cases for LLM extraction**:
- Low confidence from other extractors (< 0.7)
- Complex, multi-sentence descriptions
- Ambiguous date references requiring reasoning
- Relationship extraction between entities

**Providers**:
- Local LLMs: phi-3-mini, llama-3.2-3b (via llama.cpp)
- API: Gemini (already integrated in project)

## Implementation Plan

### Phase 1: LLM Provider Trait

**File: `extractors/src/llm_based/provider.rs`**

```rust
use serde_json::Value;
use async_trait::async_trait;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a completion from the LLM
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        config: &GenerationConfig,
    ) -> anyhow::Result<LlmResponse>;

    /// Get provider name
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub temperature: f32,
    pub max_tokens: usize,
    pub top_p: f32,
    pub force_json: bool,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: 0.1,  // Low for deterministic extraction
            max_tokens: 1000,
            top_p: 0.9,
            force_json: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}
```

### Phase 2: Gemini Provider Implementation

**File: `extractors/src/llm_based/gemini.rs`**

```rust
use super::provider::{LlmProvider, GenerationConfig, LlmResponse, TokenUsage};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "gemini-2.0-flash-exp".to_string()),
            client: Client::new(),
        }
    }

    fn build_request(&self, system_prompt: &str, user_prompt: &str, config: &GenerationConfig) -> GeminiRequest {
        let mut contents = vec![
            Content {
                role: "user".to_string(),
                parts: vec![Part {
                    text: format!("{}\n\n{}", system_prompt, user_prompt),
                }],
            }
        ];

        GeminiRequest {
            contents,
            generation_config: GeminiGenerationConfig {
                temperature: config.temperature,
                max_output_tokens: config.max_tokens as i32,
                top_p: config.top_p,
                response_mime_type: if config.force_json {
                    Some("application/json".to_string())
                } else {
                    None
                },
            },
        }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        config: &GenerationConfig,
    ) -> anyhow::Result<LlmResponse> {
        let request = self.build_request(system_prompt, user_prompt, config);

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Gemini API error: {}", error_text));
        }

        let gemini_response: GeminiResponse = response.json().await?;

        let content = gemini_response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Gemini"))?;

        let usage = gemini_response.usage_metadata
            .map(|u| TokenUsage {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.total_token_count,
            })
            .unwrap_or(TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            });

        Ok(LlmResponse {
            content,
            reasoning: None,
            usage,
        })
    }

    fn name(&self) -> &str {
        "gemini"
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    temperature: f32,
    max_output_tokens: i32,
    top_p: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<CandidatePart>,
}

#[derive(Debug, Deserialize)]
struct CandidatePart {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: usize,
    candidates_token_count: usize,
    total_token_count: usize,
}
```

### Phase 3: Prompt Templates

**File: `extractors/src/llm_based/prompts.rs`**

```rust
use shared_types::{DataType, Project, Task};

pub struct PromptTemplate {
    pub system_prompt: String,
    pub user_template: String,
    pub json_schema: serde_json::Value,
}

pub fn task_extraction_template() -> PromptTemplate {
    PromptTemplate {
        system_prompt: r#"You are a task extraction assistant. Extract task information from emails with high accuracy.

Rules for disambiguation:
- A "deadline" mentioned with "project" is a project deadline, not task due_date
- A "due date" mentioned with specific action items is a task due_date
- Dates near meeting/event language are event dates, not task deadlines
- Priority indicators: "urgent"/"ASAP"/"critical" = high, "when you can" = low
- Assignees are mentioned with "assigned to", "@username", or "can you [person]"

Only extract explicitly mentioned tasks. Do not infer or hallucinate."#.to_string(),

        user_template: r#"Extract task information from this email:

Subject: {subject}
From: {sender}
Date: {timestamp}
Body: {body}

Existing projects in database:
{existing_projects}

Return JSON with this schema:
{{
  "tasks": [{{
    "title": "string (2-100 chars, actionable)",
    "description": "string or null",
    "priority": "low|medium|high|critical",
    "due_date": "YYYY-MM-DD or null",
    "assigned_to": "string or null (person name)",
    "project_id": number or null (if matches existing project),
    "confidence": 0.0-1.0,
    "reasoning": "why you extracted this"
  }}]
}}"#.to_string(),

        json_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"},
                            "description": {"type": ["string", "null"]},
                            "priority": {"type": "string", "enum": ["low", "medium", "high", "critical"]},
                            "due_date": {"type": ["string", "null"]},
                            "assigned_to": {"type": ["string", "null"]},
                            "project_id": {"type": ["number", "null"]},
                            "confidence": {"type": "number"},
                            "reasoning": {"type": "string"}
                        },
                        "required": ["title", "confidence", "reasoning"]
                    }
                }
            },
            "required": ["tasks"]
        }),
    }
}

pub fn project_extraction_template() -> PromptTemplate {
    PromptTemplate {
        system_prompt: r#"You are a project extraction assistant. Extract project information from emails.

Rules:
- Project names are often capitalized or in quotes
- Project deadlines are different from task due dates
- Project status can be inferred from phrases like "on hold", "just started", "wrapping up"
- Don't confuse one-off tasks with ongoing projects

Only extract explicitly mentioned projects."#.to_string(),

        user_template: r#"Extract project information from this email:

Subject: {subject}
Body: {body}

Existing projects: {existing_projects}

Return JSON:
{{
  "projects": [{{
    "name": "string",
    "description": "string or null",
    "deadline": "YYYY-MM-DD or null",
    "status": "active|planning|on-hold|completed|archived or null",
    "confidence": 0.0-1.0,
    "reasoning": "why you extracted this"
  }}]
}}"#.to_string(),

        json_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "projects": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "description": {"type": ["string", "null"]},
                            "deadline": {"type": ["string", "null"]},
                            "status": {"type": ["string", "null"]},
                            "confidence": {"type": "number"},
                            "reasoning": {"type": "string"}
                        },
                        "required": ["name", "confidence", "reasoning"]
                    }
                }
            }
        }),
    }
}

pub fn event_extraction_template() -> PromptTemplate {
    PromptTemplate {
        system_prompt: r#"You are an event extraction assistant. Extract meeting and event information.

Rules:
- Event dates are when the event happens, not when the email was sent
- Parse relative dates like "tomorrow", "next Monday" based on email timestamp
- Extract all attendees mentioned
- Location can be physical or virtual (Zoom, Meet, etc.)

Only extract explicitly mentioned events."#.to_string(),

        user_template: r#"Extract event information from this email (sent on {timestamp}):

Subject: {subject}
Body: {body}

Return JSON:
{{
  "events": [{{
    "name": "string",
    "description": "string or null",
    "date": "YYYY-MM-DD",
    "time": "HH:MM or null",
    "location": "string or null",
    "attendees": ["string"],
    "project_id": number or null,
    "confidence": 0.0-1.0,
    "reasoning": "why you extracted this"
  }}]
}}"#.to_string(),

        json_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "events": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "description": {"type": ["string", "null"]},
                            "date": {"type": "string"},
                            "time": {"type": ["string", "null"]},
                            "location": {"type": ["string", "null"]},
                            "attendees": {"type": "array", "items": {"type": "string"}},
                            "project_id": {"type": ["number", "null"]},
                            "confidence": {"type": "number"},
                            "reasoning": {"type": "string"}
                        },
                        "required": ["name", "date", "confidence", "reasoning"]
                    }
                }
            }
        }),
    }
}

pub fn render_template(template: &str, vars: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}
```

### Phase 4: LLM Extractor Implementation

**File: `extractors/src/llm_based/mod.rs`**

```rust
mod provider;
mod gemini;
mod prompts;

pub use provider::{LlmProvider, GenerationConfig, LlmResponse};
pub use gemini::GeminiProvider;

use shared_types::*;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::Utc;

pub struct LlmExtractor {
    provider: Arc<dyn LlmProvider>,
    data_type: DataType,
    template: prompts::PromptTemplate,
    config: GenerationConfig,
    min_other_confidence: f32,  // Only use LLM if other methods below this
}

impl LlmExtractor {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        data_type: DataType,
        config: GenerationConfig,
    ) -> Self {
        let template = match data_type {
            DataType::Task => prompts::task_extraction_template(),
            DataType::Project => prompts::project_extraction_template(),
            DataType::Event => prompts::event_extraction_template(),
            _ => panic!("Unsupported data type for LLM extraction: {:?}", data_type),
        };

        Self {
            provider,
            data_type,
            template,
            config,
            min_other_confidence: 0.7,
        }
    }

    pub fn for_tasks(provider: Arc<dyn LlmProvider>) -> Self {
        Self::new(provider, DataType::Task, GenerationConfig::default())
    }

    pub fn for_projects(provider: Arc<dyn LlmProvider>) -> Self {
        Self::new(provider, DataType::Project, GenerationConfig::default())
    }

    pub fn for_events(provider: Arc<dyn LlmProvider>) -> Self {
        Self::new(provider, DataType::Event, GenerationConfig::default())
    }

    async fn extract_async(&self, input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError> {
        // Build prompt variables
        let mut vars = HashMap::new();
        vars.insert("subject".to_string(), input.subject.clone());
        vars.insert("sender".to_string(), input.sender.email.clone());
        vars.insert("timestamp".to_string(), input.timestamp.to_rfc3339());
        vars.insert("body".to_string(), input.body_text.clone());

        // Add existing projects context
        let existing_projects = input.existing_projects.iter()
            .map(|p| format!("- {} (ID: {})", p.name, p.id))
            .collect::<Vec<_>>()
            .join("\n");
        vars.insert("existing_projects".to_string(), existing_projects);

        // Render prompts
        let user_prompt = prompts::render_template(&self.template.user_template, &vars);

        // Call LLM
        let response = self.provider
            .generate(&self.template.system_prompt, &user_prompt, &self.config)
            .await
            .map_err(|e| ExtractionError::ModelError(e.to_string()))?;

        // Parse JSON response
        let parsed: serde_json::Value = serde_json::from_str(&response.content)
            .map_err(|e| ExtractionError::ParseError(format!("Invalid JSON: {}", e)))?;

        // Convert to ExtractionResults
        self.parse_llm_response(parsed, input)
    }

    fn parse_llm_response(
        &self,
        response: serde_json::Value,
        input: &ExtractionInput,
    ) -> Result<Vec<ExtractionResult>, ExtractionError> {
        let mut results = Vec::new();

        match self.data_type {
            DataType::Task => {
                if let Some(tasks) = response["tasks"].as_array() {
                    for task_json in tasks {
                        if let Some(result) = self.parse_task(task_json, input)? {
                            results.push(result);
                        }
                    }
                }
            }
            DataType::Project => {
                if let Some(projects) = response["projects"].as_array() {
                    for project_json in projects {
                        if let Some(result) = self.parse_project(project_json, input)? {
                            results.push(result);
                        }
                    }
                }
            }
            DataType::Event => {
                if let Some(events) = response["events"].as_array() {
                    for event_json in events {
                        if let Some(result) = self.parse_event(event_json, input)? {
                            results.push(result);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(results)
    }

    fn parse_task(&self, json: &serde_json::Value, _input: &ExtractionInput) -> Result<Option<ExtractionResult>, ExtractionError> {
        let title = json["title"].as_str()
            .ok_or_else(|| ExtractionError::ParseError("Missing task title".to_string()))?;

        let entity = ExtractedEntity::Task(ExtractedTask {
            title: title.to_string(),
            description: json["description"].as_str().map(|s| s.to_string()),
            priority: json["priority"].as_str()
                .and_then(|s| match s {
                    "low" => Some(TaskPriority::Low),
                    "medium" => Some(TaskPriority::Medium),
                    "high" => Some(TaskPriority::High),
                    "critical" => Some(TaskPriority::Critical),
                    _ => None,
                }),
            due_date: json["due_date"].as_str().map(|s| s.to_string()),
            assigned_to: json["assigned_to"].as_str().map(|s| s.to_string()),
            project_id: json["project_id"].as_i64().map(|id| id as i32),
        });

        let confidence = json["confidence"].as_f64().unwrap_or(0.7) as f32;
        let reasoning = json["reasoning"].as_str().unwrap_or("").to_string();

        let mut confidence_breakdown = HashMap::new();
        confidence_breakdown.insert("llm_extraction".to_string(), confidence);

        Ok(Some(ExtractionResult {
            entity,
            data_type: DataType::Task,
            confidence,
            confidence_breakdown,
            method: ExtractionMethod::LLMBased,
            evidence: vec![],  // Could parse from reasoning
            relationships: vec![],
            requires_review: confidence < 0.8,
            ambiguities: vec![],
            extracted_at: Utc::now(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    fn parse_project(&self, json: &serde_json::Value, _input: &ExtractionInput) -> Result<Option<ExtractionResult>, ExtractionError> {
        let name = json["name"].as_str()
            .ok_or_else(|| ExtractionError::ParseError("Missing project name".to_string()))?;

        let entity = ExtractedEntity::Project(ExtractedProject {
            name: name.to_string(),
            description: json["description"].as_str().map(|s| s.to_string()),
            deadline: json["deadline"].as_str().map(|s| s.to_string()),
            status: json["status"].as_str()
                .and_then(|s| match s {
                    "active" => Some(ProjectStatus::Active),
                    "planning" => Some(ProjectStatus::Planning),
                    "on-hold" => Some(ProjectStatus::OnHold),
                    "completed" => Some(ProjectStatus::Completed),
                    "archived" => Some(ProjectStatus::Archived),
                    _ => None,
                }),
        });

        let confidence = json["confidence"].as_f64().unwrap_or(0.7) as f32;

        let mut confidence_breakdown = HashMap::new();
        confidence_breakdown.insert("llm_extraction".to_string(), confidence);

        Ok(Some(ExtractionResult {
            entity,
            data_type: DataType::Project,
            confidence,
            confidence_breakdown,
            method: ExtractionMethod::LLMBased,
            evidence: vec![],
            relationships: vec![],
            requires_review: confidence < 0.8,
            ambiguities: vec![],
            extracted_at: Utc::now(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    fn parse_event(&self, json: &serde_json::Value, _input: &ExtractionInput) -> Result<Option<ExtractionResult>, ExtractionError> {
        let name = json["name"].as_str()
            .ok_or_else(|| ExtractionError::ParseError("Missing event name".to_string()))?;
        let date = json["date"].as_str()
            .ok_or_else(|| ExtractionError::ParseError("Missing event date".to_string()))?;

        let attendees = json["attendees"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(Vec::new);

        let entity = ExtractedEntity::Event(ExtractedEvent {
            name: name.to_string(),
            description: json["description"].as_str().map(|s| s.to_string()),
            date: date.to_string(),
            location: json["location"].as_str().map(|s| s.to_string()),
            attendees,
            project_id: json["project_id"].as_i64().map(|id| id as i32),
            task_id: None,
        });

        let confidence = json["confidence"].as_f64().unwrap_or(0.7) as f32;

        let mut confidence_breakdown = HashMap::new();
        confidence_breakdown.insert("llm_extraction".to_string(), confidence);

        Ok(Some(ExtractionResult {
            entity,
            data_type: DataType::Event,
            confidence,
            confidence_breakdown,
            method: ExtractionMethod::LLMBased,
            evidence: vec![],
            relationships: vec![],
            requires_review: confidence < 0.8,
            ambiguities: vec![],
            extracted_at: Utc::now(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }
}

impl Extractor for LlmExtractor {
    fn extract(&self, input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError> {
        // Use tokio runtime to run async code
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| ExtractionError::ModelError(e.to_string()))?;

        runtime.block_on(self.extract_async(input))
    }

    fn data_type(&self) -> DataType {
        self.data_type
    }

    fn method(&self) -> ExtractionMethod {
        ExtractionMethod::LLMBased
    }
}
```

### Phase 5: Update Dependencies

**File: `extractors/Cargo.toml`**

```toml
[dependencies]
# ... existing dependencies

# Async runtime
tokio = { workspace = true, features = ["full"] }
async-trait = "0.1"
reqwest = { version = "0.11", features = ["json"] }
```

### Phase 6: Testing

**File: `extractors/tests/llm_extractor_test.rs`**

```rust
use extractors::llm_based::*;
use shared_types::*;
use std::sync::Arc;

#[tokio::test]
#[ignore]  // Requires API key
async fn test_llm_task_extraction() {
    let api_key = std::env::var("GEMINI_API_KEY").unwrap();
    let provider = Arc::new(GeminiProvider::new(api_key, None));

    let extractor = LlmExtractor::for_tasks(provider);

    let input = create_test_input(
        "Action Items from Meeting",
        "Please complete the homepage mockup by Friday. This is urgent. @Sarah, can you handle this?"
    );

    let results = extractor.extract(&input).unwrap();
    assert!(!results.is_empty());

    let result = &results[0];
    match &result.entity {
        ExtractedEntity::Task(task) => {
            assert!(task.title.contains("mockup"));
            assert_eq!(task.priority, Some(TaskPriority::High));
            assert_eq!(task.assigned_to, Some("Sarah".to_string()));
        }
        _ => panic!("Expected Task entity"),
    }
}
```

### Phase 7: Update Library

**File: `extractors/src/lib.rs`**

```rust
pub mod attachment_parser;
pub mod pattern_based;
pub mod gliner;
pub mod llm_based;

pub use attachment_parser::{
    AttachmentParserExtractor,
    IcsParserConfig,
    TimezoneHandling,
};

pub use pattern_based::{
    PatternExtractor,
    ExtractionPattern,
};

pub use gliner::{
    GlinerExtractor,
    GlinerConfig,
};

pub use llm_based::{
    LlmExtractor,
    LlmProvider,
    GeminiProvider,
    GenerationConfig,
};
```

## Usage Example

```rust
use extractors::*;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create Gemini provider
    let api_key = std::env::var("GEMINI_API_KEY").unwrap();
    let provider = Arc::new(GeminiProvider::new(api_key, None));

    // Create LLM extractor for tasks
    let extractor = LlmExtractor::for_tasks(provider);

    // Extract from email
    let input = create_extraction_input(...);
    let results = extractor.extract(&input).unwrap();

    for result in results {
        println!("Extracted: {:?} (confidence: {})", result.entity, result.confidence);
    }
}
```

## Cost Optimization

- Only use LLM when other extractors have low confidence
- Batch multiple extractions in one call
- Use smaller, cheaper models for simple cases
- Cache prompts where possible
- Track API costs per extraction

## Future Enhancements

1. Add local LLM support via llama.cpp
2. Implement prompt caching
3. Add few-shot examples from user corrections
4. Implement chain-of-thought reasoning
5. Add multi-turn extraction for complex emails
6. Implement confidence calibration based on past accuracy

## References

- Gemini API: https://ai.google.dev/api
- Anthropic Claude API: https://docs.anthropic.com/
- llama.cpp Rust bindings: https://github.com/utilityai/llama-cpp-rs
- Prompt engineering guide: https://www.promptingguide.ai/
