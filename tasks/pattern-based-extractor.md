# Implement Pattern/Rule-Based Extractor

## Objective

Implement pattern-based extraction using regex and keyword matching for common email patterns. This provides fast, deterministic extraction for well-known formats and phrases.

**Confidence Target**: 0.7-0.9 for well-matched patterns

## Dependencies

**Blocked by**: Task #1 - Extraction Framework Foundation

Requires the `Extractor` trait and common types from shared-types.

## Background

Emails often contain predictable patterns for expressing tasks, deadlines, priorities, and project information:
- Due dates: "due by Jan 25", "deadline: Friday", "complete by EOD"
- Priorities: "urgent", "ASAP", "high priority", "critical"
- Assignments: "assigned to Sarah", "@mike can you", "please handle this"
- Project mentions: "[Project Alpha]", "for the website redesign project"

Pattern-based extraction is:
- **Fast**: Regex matching is very efficient
- **Deterministic**: Same input always produces same output
- **Transparent**: Easy to understand why something was extracted
- **Maintainable**: Patterns can be updated without retraining models

## Implementation Plan

### Phase 1: Core Pattern Engine

**File: `extractors/src/pattern_based/mod.rs`**

```rust
mod patterns;
mod date_parser;

pub use patterns::*;
pub use date_parser::DateParser;

use shared_types::{
    ExtractionInput, ExtractionResult, ExtractionError,
    DataType, ExtractionMethod, Extractor,
};
use regex::Regex;
use std::collections::HashMap;

pub struct PatternExtractor {
    patterns: Vec<ExtractionPattern>,
    date_parser: DateParser,
    context_window: usize,
}

impl PatternExtractor {
    pub fn new(patterns: Vec<ExtractionPattern>, context_window: usize) -> Self {
        Self {
            patterns,
            date_parser: DateParser::new(),
            context_window,
        }
    }

    /// Create a task-focused pattern extractor
    pub fn for_tasks() -> Self {
        Self::new(patterns::task_patterns(), 50)
    }

    /// Create a project-focused pattern extractor
    pub fn for_projects() -> Self {
        Self::new(patterns::project_patterns(), 50)
    }

    /// Create an event-focused pattern extractor
    pub fn for_events() -> Self {
        Self::new(patterns::event_patterns(), 50)
    }

    fn extract_with_patterns(&self, input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError> {
        let text = format!("{}\n\n{}", input.subject, input.body_text);
        let mut results = Vec::new();

        for pattern in &self.patterns {
            if let Some(mut extracted) = pattern.apply(&text, input, self.context_window)? {
                // Filter by confidence threshold
                if extracted.confidence >= input.min_confidence {
                    results.push(extracted);
                }
            }
        }

        // Apply max results limit
        if let Some(max) = input.max_results {
            results.truncate(max);
        }

        Ok(results)
    }
}

impl Extractor for PatternExtractor {
    fn extract(&self, input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError> {
        self.extract_with_patterns(input)
    }

    fn data_type(&self) -> DataType {
        // Determined by patterns loaded
        self.patterns.first()
            .map(|p| p.target_data_type)
            .unwrap_or(DataType::Task)
    }

    fn method(&self) -> ExtractionMethod {
        ExtractionMethod::PatternBased
    }
}
```

### Phase 2: Pattern Definition

**File: `extractors/src/pattern_based/patterns.rs`**

```rust
use regex::Regex;
use shared_types::*;
use std::collections::HashMap;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct ExtractionPattern {
    pub name: String,
    pub target_data_type: DataType,
    pub target_field: String,
    pub confidence: f32,
    pub pattern_type: PatternType,
    pub must_appear_with: Vec<String>,
    pub must_not_appear_with: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    Regex(Regex),
    Keywords(Vec<String>),
    Combined(Regex, Vec<String>),
}

impl ExtractionPattern {
    pub fn apply(
        &self,
        text: &str,
        input: &ExtractionInput,
        context_window: usize,
    ) -> Result<Option<ExtractionResult>, ExtractionError> {
        match &self.pattern_type {
            PatternType::Regex(regex) => self.apply_regex(regex, text, input, context_window),
            PatternType::Keywords(keywords) => self.apply_keywords(keywords, text, input, context_window),
            PatternType::Combined(regex, keywords) => {
                // Both must match
                if let Some(result) = self.apply_regex(regex, text, input, context_window)? {
                    if self.check_keywords(keywords, text) {
                        return Ok(Some(result));
                    }
                }
                Ok(None)
            }
        }
    }

    fn apply_regex(
        &self,
        regex: &Regex,
        text: &str,
        input: &ExtractionInput,
        context_window: usize,
    ) -> Result<Option<ExtractionResult>, ExtractionError> {
        for capture in regex.captures_iter(text) {
            if let Some(matched) = capture.get(0) {
                let start = matched.start();
                let end = matched.end();

                // Extract context around match
                let context_start = start.saturating_sub(context_window);
                let context_end = (end + context_window).min(text.len());
                let context = &text[context_start..context_end];

                // Check context requirements
                if !self.check_context_requirements(context) {
                    continue;
                }

                // Extract value (from capture group 1 if exists, else full match)
                let value = capture.get(1)
                    .or_else(|| capture.get(0))
                    .map(|m| m.as_str().to_string())
                    .ok_or_else(|| ExtractionError::ParseError("No match found".to_string()))?;

                // Build extraction result based on target type
                return self.build_result(value, text, start, end, input);
            }
        }

        Ok(None)
    }

    fn apply_keywords(
        &self,
        keywords: &[String],
        text: &str,
        input: &ExtractionInput,
        context_window: usize,
    ) -> Result<Option<ExtractionResult>, ExtractionError> {
        let text_lower = text.to_lowercase();

        for keyword in keywords {
            if let Some(pos) = text_lower.find(&keyword.to_lowercase()) {
                let start = pos;
                let end = pos + keyword.len();

                // Extract context
                let context_start = start.saturating_sub(context_window);
                let context_end = (end + context_window).min(text.len());
                let context = &text[context_start..context_end];

                if !self.check_context_requirements(context) {
                    continue;
                }

                return self.build_result(keyword.clone(), text, start, end, input);
            }
        }

        Ok(None)
    }

    fn check_keywords(&self, keywords: &[String], text: &str) -> bool {
        let text_lower = text.to_lowercase();
        keywords.iter().any(|kw| text_lower.contains(&kw.to_lowercase()))
    }

    fn check_context_requirements(&self, context: &str) -> bool {
        let context_lower = context.to_lowercase();

        // Check must_appear_with
        if !self.must_appear_with.is_empty() {
            let has_required = self.must_appear_with.iter()
                .any(|req| context_lower.contains(&req.to_lowercase()));
            if !has_required {
                return false;
            }
        }

        // Check must_not_appear_with
        for negation in &self.must_not_appear_with {
            if context_lower.contains(&negation.to_lowercase()) {
                return false;
            }
        }

        true
    }

    fn build_result(
        &self,
        value: String,
        text: &str,
        start: usize,
        end: usize,
        input: &ExtractionInput,
    ) -> Result<Option<ExtractionResult>, ExtractionError> {
        // Build entity based on target_data_type and target_field
        let entity = match self.target_data_type {
            DataType::Task => self.build_task_entity(value, input),
            DataType::Project => self.build_project_entity(value, input),
            DataType::Priority => return Ok(None),  // Priority is a field, not an entity
            _ => return Ok(None),
        };

        let evidence = vec![TextSpan {
            source: TextSource::Body,
            start,
            end,
            text: text[start..end].to_string(),
        }];

        let mut confidence_breakdown = HashMap::new();
        confidence_breakdown.insert(self.target_field.clone(), self.confidence);

        Ok(Some(ExtractionResult {
            entity,
            data_type: self.target_data_type,
            confidence: self.confidence,
            confidence_breakdown,
            method: ExtractionMethod::PatternBased,
            evidence,
            relationships: Vec::new(),
            requires_review: self.confidence < 0.8,
            ambiguities: Vec::new(),
            extracted_at: Utc::now(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    fn build_task_entity(&self, value: String, _input: &ExtractionInput) -> ExtractedEntity {
        match self.target_field.as_str() {
            "title" => ExtractedEntity::Task(ExtractedTask {
                title: value,
                description: None,
                priority: None,
                due_date: None,
                assigned_to: None,
                project_id: None,
            }),
            "due_date" => {
                // Parse date from value
                ExtractedEntity::Task(ExtractedTask {
                    title: "Extracted task".to_string(),
                    description: None,
                    priority: None,
                    due_date: Some(value),
                    assigned_to: None,
                    project_id: None,
                })
            }
            _ => ExtractedEntity::Task(ExtractedTask {
                title: value,
                description: None,
                priority: None,
                due_date: None,
                assigned_to: None,
                project_id: None,
            }),
        }
    }

    fn build_project_entity(&self, value: String, _input: &ExtractionInput) -> ExtractedEntity {
        ExtractedEntity::Project(ExtractedProject {
            name: value,
            description: None,
            deadline: None,
            status: None,
        })
    }
}

/// Create task-specific patterns
pub fn task_patterns() -> Vec<ExtractionPattern> {
    vec![
        // Due date patterns
        ExtractionPattern {
            name: "task_due_date_explicit".to_string(),
            target_data_type: DataType::Task,
            target_field: "due_date".to_string(),
            confidence: 0.9,
            pattern_type: PatternType::Regex(
                Regex::new(r"(?i)(due|deadline|complete by|finish by)[:\s]+([A-Za-z]+ \d{1,2}|\d{1,2}/\d{1,2}/\d{4})").unwrap()
            ),
            must_appear_with: vec!["task".to_string(), "action".to_string(), "complete".to_string()],
            must_not_appear_with: vec!["project deadline".to_string()],
        },

        // Priority patterns
        ExtractionPattern {
            name: "high_priority_keywords".to_string(),
            target_data_type: DataType::Priority,
            target_field: "priority".to_string(),
            confidence: 0.85,
            pattern_type: PatternType::Keywords(vec![
                "urgent".to_string(),
                "ASAP".to_string(),
                "high priority".to_string(),
                "critical".to_string(),
                "important".to_string(),
            ]),
            must_appear_with: vec!["task".to_string(), "action".to_string(), "need".to_string()],
            must_not_appear_with: vec!["not urgent".to_string(), "low priority".to_string()],
        },

        // Assignment patterns
        ExtractionPattern {
            name: "task_assignment".to_string(),
            target_data_type: DataType::Task,
            target_field: "assigned_to".to_string(),
            confidence: 0.8,
            pattern_type: PatternType::Regex(
                Regex::new(r"(?i)assigned to:?\s+([A-Za-z]+)").unwrap()
            ),
            must_appear_with: vec![],
            must_not_appear_with: vec!["from".to_string()],
        },

        // Action item patterns
        ExtractionPattern {
            name: "numbered_action_item".to_string(),
            target_data_type: DataType::Task,
            target_field: "title".to_string(),
            confidence: 0.75,
            pattern_type: PatternType::Regex(
                Regex::new(r"(?m)^\d+\.\s+(.+)$").unwrap()
            ),
            must_appear_with: vec![],
            must_not_appear_with: vec![],
        },
    ]
}

/// Create project-specific patterns
pub fn project_patterns() -> Vec<ExtractionPattern> {
    vec![
        ExtractionPattern {
            name: "project_mention_brackets".to_string(),
            target_data_type: DataType::Project,
            target_field: "name".to_string(),
            confidence: 0.85,
            pattern_type: PatternType::Regex(
                Regex::new(r"\[([A-Z][A-Za-z\s]+)\]").unwrap()
            ),
            must_appear_with: vec![],
            must_not_appear_with: vec![],
        },

        ExtractionPattern {
            name: "project_keyword".to_string(),
            target_data_type: DataType::Project,
            target_field: "name".to_string(),
            confidence: 0.75,
            pattern_type: PatternType::Regex(
                Regex::new(r"(?i)(?:for the|regarding the|about the)\s+([A-Z][A-Za-z\s]+)\s+project").unwrap()
            ),
            must_appear_with: vec![],
            must_not_appear_with: vec![],
        },
    ]
}

/// Create event-specific patterns
pub fn event_patterns() -> Vec<ExtractionPattern> {
    vec![
        ExtractionPattern {
            name: "meeting_mention".to_string(),
            target_data_type: DataType::Event,
            target_field: "name".to_string(),
            confidence: 0.8,
            pattern_type: PatternType::Regex(
                Regex::new(r"(?i)(meeting|call|sync|standup|review)\s+(?:on|at)?\s+([A-Za-z]+ \d{1,2})").unwrap()
            ),
            must_appear_with: vec![],
            must_not_appear_with: vec![],
        },
    ]
}
```

### Phase 3: Date Parser

**File: `extractors/src/pattern_based/date_parser.rs`**

```rust
use chrono::{DateTime, Utc, Duration, Datelike};
use regex::Regex;

pub struct DateParser {
    absolute_patterns: Vec<Regex>,
    relative_patterns: Vec<(Regex, RelativeDateHandler)>,
}

type RelativeDateHandler = fn(DateTime<Utc>) -> DateTime<Utc>;

impl DateParser {
    pub fn new() -> Self {
        Self {
            absolute_patterns: vec![
                Regex::new(r"(\d{1,2})/(\d{1,2})/(\d{4})").unwrap(),  // MM/DD/YYYY
                Regex::new(r"([A-Z][a-z]+)\s+(\d{1,2})").unwrap(),     // Month Day
            ],
            relative_patterns: vec![
                (Regex::new(r"(?i)tomorrow").unwrap(), |now| now + Duration::days(1)),
                (Regex::new(r"(?i)next week").unwrap(), |now| now + Duration::weeks(1)),
                (Regex::new(r"(?i)in (\d+) days?").unwrap(), |now| now + Duration::days(2)), // Placeholder
            ],
        }
    }

    pub fn parse(&self, text: &str, reference_date: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Try absolute patterns first
        for pattern in &self.absolute_patterns {
            if let Some(date) = self.try_absolute_pattern(pattern, text, reference_date) {
                return Some(date);
            }
        }

        // Try relative patterns
        for (pattern, handler) in &self.relative_patterns {
            if pattern.is_match(text) {
                return Some(handler(reference_date));
            }
        }

        None
    }

    fn try_absolute_pattern(
        &self,
        pattern: &Regex,
        text: &str,
        reference_date: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        // Simplified - real implementation would handle various date formats
        None
    }
}
```

### Phase 4: Testing

**File: `extractors/src/pattern_based/mod.rs`**

Add tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::*;
    use chrono::Utc;

    fn create_test_input(subject: &str, body: &str) -> ExtractionInput {
        ExtractionInput {
            email_id: "test".to_string(),
            subject: subject.to_string(),
            body_text: body.to_string(),
            body_html: None,
            attachments: vec![],
            sender: EmailAddress {
                email: "test@example.com".to_string(),
                name: None,
            },
            recipients: vec![],
            timestamp: Utc::now(),
            thread_id: None,
            in_reply_to: None,
            extracted_entities: vec![],
            existing_projects: vec![],
            existing_tasks: vec![],
            existing_contacts: vec![],
            user_timezone: "UTC".to_string(),
            user_language: "en".to_string(),
            user_preferences: UserPreferences {
                date_format: "YYYY-MM-DD".to_string(),
                default_task_priority: TaskPriority::Medium,
                default_project_status: ProjectStatus::Active,
                auto_link_threshold: 0.8,
            },
            target_data_type: DataType::Task,
            min_confidence: 0.5,
            max_results: None,
        }
    }

    #[test]
    fn test_task_due_date_pattern() {
        let extractor = PatternExtractor::for_tasks();
        let input = create_test_input(
            "Action Items",
            "Please complete the task due by Jan 25"
        );

        let results = extractor.extract(&input).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].confidence >= 0.7);
    }

    #[test]
    fn test_high_priority_pattern() {
        let extractor = PatternExtractor::for_tasks();
        let input = create_test_input(
            "Urgent Task",
            "This task needs urgent attention and action"
        );

        let results = extractor.extract(&input).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_project_mention_pattern() {
        let extractor = PatternExtractor::for_projects();
        let input = create_test_input(
            "Update",
            "For the Website Redesign project, we need..."
        );

        let results = extractor.extract(&input).unwrap();
        assert!(!results.is_empty());
    }
}
```

### Phase 5: Update Library

**File: `extractors/src/lib.rs`**

```rust
pub mod attachment_parser;
pub mod pattern_based;

pub use attachment_parser::{
    AttachmentParserExtractor,
    IcsParserConfig,
    TimezoneHandling,
};

pub use pattern_based::{
    PatternExtractor,
    ExtractionPattern,
};
```

## Testing Strategy

1. **Unit tests**: Test individual patterns in isolation
2. **Integration tests**: Test full extraction pipeline
3. **Real email tests**: Use anonymized real email samples
4. **Edge cases**: Negations, ambiguous contexts, multiple matches

```bash
cd extractors
cargo test pattern_based
```

## Pattern Library Expansion

Future patterns to add:
- Email signatures for contact extraction
- Project status indicators ("on hold", "blocked")
- Time expressions ("2 weeks", "Q1 2026")
- Location patterns ("at office", "via Zoom")
- More date formats (ISO 8601, European format)

## Next Steps

1. Add more comprehensive date parsing (use `dateparser` crate)
2. Create pattern library from real email corpus
3. Add pattern priority/conflict resolution
4. Implement pattern learning from user corrections
5. Add support for multiple languages

## References

- Regex crate: https://docs.rs/regex/
- dateparser crate: https://crates.io/crates/dateparser
- chrono: https://docs.rs/chrono/
