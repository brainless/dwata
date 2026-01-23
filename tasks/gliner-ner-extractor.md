# Implement GLiNER NER Extractor

## Objective

Implement GLiNER-based Named Entity Recognition extractor for flexible, zero-shot entity extraction from email text. GLiNER enables extracting custom entity types without training, making it ideal for extracting projects, tasks, events, and other entities.

**Confidence Target**: 0.6-0.85 depending on entity clarity

## Dependencies

**Blocked by**: Task #1 - Extraction Framework Foundation

Requires the `Extractor` trait and common types from shared-types.

## Background

GLiNER (Generalist and Lightweight Named Entity Recognition) is a zero-shot NER model that can extract arbitrary entity types defined at inference time. Unlike traditional NER models trained on fixed entity types (PERSON, ORG, LOC), GLiNER can extract custom entities like:
- PROJECT_NAME
- TASK_TITLE
- DUE_DATE
- ASSIGNEE
- EVENT_NAME

**Advantages**:
- No training data required
- Flexible entity types
- Runs efficiently on CPU via ONNX
- Good accuracy for common entity patterns

**Model variants**:
- gliner-small-v2.1: Fast, less accurate
- gliner-medium-v2.1: Balanced (recommended)
- gliner-large-v2.1: Most accurate, slower

## Implementation Plan

### Phase 1: Dependencies

**File: `extractors/Cargo.toml`**

Add dependencies:

```toml
[dependencies]
# ... existing dependencies

# ONNX Runtime for GLiNER
ort = "2.0"
tokenizers = "0.15"
ndarray = "0.15"
```

### Phase 2: GLiNER Model Wrapper

**File: `extractors/src/gliner/model.rs`**

```rust
use ort::{Session, SessionBuilder, Value, ValueType};
use tokenizers::Tokenizer;
use ndarray::{Array1, Array2};
use std::path::Path;

pub struct GlinerModel {
    session: Session,
    tokenizer: Tokenizer,
    max_length: usize,
}

impl GlinerModel {
    pub fn new(model_path: &Path, tokenizer_path: &Path) -> anyhow::Result<Self> {
        // Load ONNX model
        let session = SessionBuilder::new()?
            .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self {
            session,
            tokenizer,
            max_length: 512,
        })
    }

    pub fn extract_entities(
        &self,
        text: &str,
        entity_labels: &[String],
        threshold: f32,
    ) -> anyhow::Result<Vec<ExtractedSpan>> {
        // Tokenize input
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();

        // Truncate to max_length
        let seq_len = input_ids.len().min(self.max_length);
        let input_ids = &input_ids[..seq_len];
        let attention_mask = &attention_mask[..seq_len];

        // Prepare entity label encodings
        let entity_encodings: Vec<_> = entity_labels
            .iter()
            .map(|label| self.tokenizer.encode(label.as_str(), false))
            .collect::<Result<_, _>>()
            .map_err(|e| anyhow::anyhow!("Label encoding failed: {}", e))?;

        // Convert to arrays
        let input_ids_array = Array2::from_shape_vec(
            (1, seq_len),
            input_ids.iter().map(|&id| id as i64).collect(),
        )?;

        let attention_mask_array = Array2::from_shape_vec(
            (1, seq_len),
            attention_mask.iter().map(|&m| m as i64).collect(),
        )?;

        // Run inference
        let outputs = self.session.run(ort::inputs![
            "input_ids" => Value::from_array(input_ids_array)?,
            "attention_mask" => Value::from_array(attention_mask_array)?,
        ]?)?;

        // Parse outputs
        let logits = outputs["logits"]
            .try_extract_tensor::<f32>()?
            .view()
            .to_owned();

        // Post-process to extract entity spans
        self.decode_entities(&logits, &encoding, entity_labels, threshold)
    }

    fn decode_entities(
        &self,
        logits: &ndarray::ArrayView3<f32>,
        encoding: &tokenizers::Encoding,
        entity_labels: &[String],
        threshold: f32,
    ) -> anyhow::Result<Vec<ExtractedSpan>> {
        let mut spans = Vec::new();

        // Simplified decoding - real implementation would use proper BIO tagging
        // For each token, if score > threshold, add to spans
        for (token_idx, offsets) in encoding.get_offsets().iter().enumerate() {
            for (label_idx, label) in entity_labels.iter().enumerate() {
                let score = logits[[0, token_idx, label_idx]];

                if score > threshold {
                    spans.push(ExtractedSpan {
                        text: encoding.get_tokens()[token_idx].clone(),
                        label: label.clone(),
                        start: offsets.0,
                        end: offsets.1,
                        score,
                    });
                }
            }
        }

        // Merge adjacent tokens of same entity type
        let merged = self.merge_adjacent_spans(spans);

        Ok(merged)
    }

    fn merge_adjacent_spans(&self, spans: Vec<ExtractedSpan>) -> Vec<ExtractedSpan> {
        if spans.is_empty() {
            return spans;
        }

        let mut merged = Vec::new();
        let mut current = spans[0].clone();

        for span in spans.into_iter().skip(1) {
            if span.label == current.label && span.start <= current.end + 2 {
                // Adjacent or very close - merge
                current.end = span.end;
                current.text = format!("{} {}", current.text, span.text);
                current.score = (current.score + span.score) / 2.0;
            } else {
                merged.push(current);
                current = span;
            }
        }
        merged.push(current);

        merged
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedSpan {
    pub text: String,
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub score: f32,
}
```

### Phase 3: Entity Rules for Disambiguation

**File: `extractors/src/gliner/rules.rs`**

```rust
use regex::Regex;

#[derive(Debug, Clone)]
pub struct EntityRule {
    pub label: String,
    pub context_keywords: Vec<String>,
    pub negation_keywords: Vec<String>,
    pub required_pattern: Option<Regex>,
}

impl EntityRule {
    pub fn adjust_confidence(&self, text: &str, context: &str, base_confidence: f32) -> f32 {
        let mut confidence = base_confidence;

        // Boost if context keywords present
        if !self.context_keywords.is_empty() {
            let context_lower = context.to_lowercase();
            let has_context = self.context_keywords.iter()
                .any(|kw| context_lower.contains(&kw.to_lowercase()));

            if has_context {
                confidence *= 1.2;  // 20% boost
            } else {
                confidence *= 0.8;  // 20% penalty if expected context missing
            }
        }

        // Penalize if negation keywords present
        if !self.negation_keywords.is_empty() {
            let context_lower = context.to_lowercase();
            let has_negation = self.negation_keywords.iter()
                .any(|kw| context_lower.contains(&kw.to_lowercase()));

            if has_negation {
                confidence *= 0.3;  // Strong penalty for negation
            }
        }

        // Check required pattern
        if let Some(ref pattern) = self.required_pattern {
            if !pattern.is_match(text) {
                confidence *= 0.5;  // Doesn't match expected format
            }
        }

        confidence.min(1.0)
    }
}

pub fn task_entity_rules() -> Vec<EntityRule> {
    vec![
        EntityRule {
            label: "DUE_DATE".to_string(),
            context_keywords: vec!["task".to_string(), "complete".to_string(), "finish".to_string()],
            negation_keywords: vec!["project deadline".to_string(), "event date".to_string()],
            required_pattern: None,
        },
        EntityRule {
            label: "ASSIGNEE".to_string(),
            context_keywords: vec!["assigned to".to_string(), "@".to_string(), "can you".to_string()],
            negation_keywords: vec!["from".to_string(), "sender".to_string()],
            required_pattern: None,
        },
        EntityRule {
            label: "TASK_TITLE".to_string(),
            context_keywords: vec!["action".to_string(), "need to".to_string(), "must".to_string()],
            negation_keywords: vec![],
            required_pattern: None,
        },
    ]
}

pub fn project_entity_rules() -> Vec<EntityRule> {
    vec![
        EntityRule {
            label: "PROJECT_NAME".to_string(),
            context_keywords: vec!["project".to_string(), "initiative".to_string()],
            negation_keywords: vec![],
            required_pattern: None,
        },
        EntityRule {
            label: "PROJECT_DEADLINE".to_string(),
            context_keywords: vec!["project".to_string(), "delivery".to_string()],
            negation_keywords: vec!["task due".to_string()],
            required_pattern: None,
        },
    ]
}

pub fn event_entity_rules() -> Vec<EntityRule> {
    vec![
        EntityRule {
            label: "EVENT_NAME".to_string(),
            context_keywords: vec!["meeting".to_string(), "call".to_string(), "sync".to_string()],
            negation_keywords: vec![],
            required_pattern: None,
        },
        EntityRule {
            label: "EVENT_DATE".to_string(),
            context_keywords: vec!["meeting".to_string(), "event".to_string(), "scheduled".to_string()],
            negation_keywords: vec!["task due".to_string(), "project deadline".to_string()],
            required_pattern: None,
        },
    ]
}
```

### Phase 4: GLiNER Extractor Implementation

**File: `extractors/src/gliner/mod.rs`**

```rust
mod model;
mod rules;

pub use model::{GlinerModel, ExtractedSpan};
pub use rules::EntityRule;

use shared_types::{
    ExtractionInput, ExtractionResult, ExtractionError,
    DataType, ExtractionMethod, Extractor, ExtractedEntity,
    ExtractedTask, ExtractedProject, ExtractedEvent,
    TextSpan, TextSource,
};
use std::path::PathBuf;
use std::collections::HashMap;
use chrono::Utc;

pub struct GlinerExtractor {
    model: GlinerModel,
    data_type: DataType,
    entity_labels: Vec<String>,
    entity_rules: Vec<EntityRule>,
    threshold: f32,
    context_window: usize,
}

pub struct GlinerConfig {
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub data_type: DataType,
    pub threshold: f32,
    pub context_window: usize,
}

impl GlinerExtractor {
    pub fn new(config: GlinerConfig) -> anyhow::Result<Self> {
        let model = GlinerModel::new(&config.model_path, &config.tokenizer_path)?;

        // Select entity labels and rules based on data type
        let (entity_labels, entity_rules) = match config.data_type {
            DataType::Task => (
                vec![
                    "TASK_TITLE".to_string(),
                    "TASK_DESCRIPTION".to_string(),
                    "DUE_DATE".to_string(),
                    "ASSIGNEE".to_string(),
                    "PRIORITY_INDICATOR".to_string(),
                ],
                rules::task_entity_rules(),
            ),
            DataType::Project => (
                vec![
                    "PROJECT_NAME".to_string(),
                    "PROJECT_DESCRIPTION".to_string(),
                    "PROJECT_DEADLINE".to_string(),
                    "PROJECT_STATUS".to_string(),
                ],
                rules::project_entity_rules(),
            ),
            DataType::Event => (
                vec![
                    "EVENT_NAME".to_string(),
                    "EVENT_DATE".to_string(),
                    "EVENT_TIME".to_string(),
                    "EVENT_LOCATION".to_string(),
                    "ATTENDEE_NAME".to_string(),
                ],
                rules::event_entity_rules(),
            ),
            _ => return Err(anyhow::anyhow!("Unsupported data type for GLiNER: {:?}", config.data_type)),
        };

        Ok(Self {
            model,
            data_type: config.data_type,
            entity_labels,
            entity_rules,
            threshold: config.threshold,
            context_window: config.context_window,
        })
    }

    pub fn for_tasks(model_path: PathBuf, tokenizer_path: PathBuf) -> anyhow::Result<Self> {
        Self::new(GlinerConfig {
            model_path,
            tokenizer_path,
            data_type: DataType::Task,
            threshold: 0.6,
            context_window: 50,
        })
    }

    pub fn for_projects(model_path: PathBuf, tokenizer_path: PathBuf) -> anyhow::Result<Self> {
        Self::new(GlinerConfig {
            model_path,
            tokenizer_path,
            data_type: DataType::Project,
            threshold: 0.65,
            context_window: 50,
        })
    }

    fn extract_from_text(&self, text: &str) -> anyhow::Result<Vec<ExtractionResult>> {
        // Run GLiNER model
        let spans = self.model.extract_entities(text, &self.entity_labels, self.threshold)?;

        let mut results = Vec::new();

        for span in spans {
            // Get context around span
            let context_start = span.start.saturating_sub(self.context_window);
            let context_end = (span.end + self.context_window).min(text.len());
            let context = &text[context_start..context_end];

            // Apply entity rules to adjust confidence
            let adjusted_confidence = if let Some(rule) = self.entity_rules.iter().find(|r| r.label == span.label) {
                rule.adjust_confidence(&span.text, context, span.score)
            } else {
                span.score
            };

            // Skip if confidence too low after adjustment
            if adjusted_confidence < self.threshold {
                continue;
            }

            // Build entity from span
            if let Some(entity) = self.build_entity_from_span(&span)? {
                let evidence = vec![TextSpan {
                    source: TextSource::Body,
                    start: span.start,
                    end: span.end,
                    text: span.text.clone(),
                }];

                let mut confidence_breakdown = HashMap::new();
                confidence_breakdown.insert(span.label.clone(), adjusted_confidence);

                results.push(ExtractionResult {
                    entity,
                    data_type: self.data_type,
                    confidence: adjusted_confidence,
                    confidence_breakdown,
                    method: ExtractionMethod::GlinerNER,
                    evidence,
                    relationships: Vec::new(),
                    requires_review: adjusted_confidence < 0.75,
                    ambiguities: Vec::new(),
                    extracted_at: Utc::now(),
                    extractor_version: env!("CARGO_PKG_VERSION").to_string(),
                });
            }
        }

        Ok(results)
    }

    fn build_entity_from_span(&self, span: &ExtractedSpan) -> anyhow::Result<Option<ExtractedEntity>> {
        let entity = match self.data_type {
            DataType::Task => match span.label.as_str() {
                "TASK_TITLE" => Some(ExtractedEntity::Task(ExtractedTask {
                    title: span.text.clone(),
                    description: None,
                    priority: None,
                    due_date: None,
                    assigned_to: None,
                    project_id: None,
                })),
                "DUE_DATE" => Some(ExtractedEntity::Task(ExtractedTask {
                    title: "Task with due date".to_string(),
                    description: None,
                    priority: None,
                    due_date: Some(span.text.clone()),
                    assigned_to: None,
                    project_id: None,
                })),
                _ => None,
            },
            DataType::Project => match span.label.as_str() {
                "PROJECT_NAME" => Some(ExtractedEntity::Project(ExtractedProject {
                    name: span.text.clone(),
                    description: None,
                    deadline: None,
                    status: None,
                })),
                _ => None,
            },
            DataType::Event => match span.label.as_str() {
                "EVENT_NAME" => Some(ExtractedEntity::Event(ExtractedEvent {
                    name: span.text.clone(),
                    description: None,
                    date: String::new(),
                    location: None,
                    attendees: Vec::new(),
                    project_id: None,
                    task_id: None,
                })),
                _ => None,
            },
            _ => None,
        };

        Ok(entity)
    }
}

impl Extractor for GlinerExtractor {
    fn extract(&self, input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError> {
        let text = format!("{}\n\n{}", input.subject, input.body_text);

        self.extract_from_text(&text)
            .map_err(|e| ExtractionError::ModelError(e.to_string()))
    }

    fn data_type(&self) -> DataType {
        self.data_type
    }

    fn method(&self) -> ExtractionMethod {
        ExtractionMethod::GlinerNER
    }
}
```

### Phase 5: Testing

**File: `extractors/tests/gliner_test.rs`**

```rust
use extractors::gliner::*;
use shared_types::*;
use std::path::PathBuf;

#[test]
#[ignore]  // Requires model files
fn test_gliner_task_extraction() {
    let model_path = PathBuf::from("models/gliner-medium-v2.1.onnx");
    let tokenizer_path = PathBuf::from("models/tokenizer.json");

    let extractor = GlinerExtractor::for_tasks(model_path, tokenizer_path).unwrap();

    let input = create_test_input(
        "Action Items",
        "Please complete the homepage mockup by Friday. This is urgent."
    );

    let results = extractor.extract(&input).unwrap();
    assert!(!results.is_empty());
}

fn create_test_input(subject: &str, body: &str) -> ExtractionInput {
    // ... same as pattern_based tests
}
```

### Phase 6: Model Download Script

**File: `scripts/download_gliner_model.sh`**

```bash
#!/bin/bash

set -e

MODEL_DIR="models"
mkdir -p "$MODEL_DIR"

echo "Downloading GLiNER medium model..."

# Download from Hugging Face or ONNX model zoo
# This is a placeholder - actual URLs would be provided
wget -O "$MODEL_DIR/gliner-medium-v2.1.onnx" \
    "https://example.com/gliner-medium-v2.1.onnx"

wget -O "$MODEL_DIR/tokenizer.json" \
    "https://example.com/tokenizer.json"

echo "GLiNER model downloaded to $MODEL_DIR"
```

## Setup Instructions

1. Download GLiNER model:
   ```bash
   chmod +x scripts/download_gliner_model.sh
   ./scripts/download_gliner_model.sh
   ```

2. Build extractors:
   ```bash
   cd extractors
   cargo build --release
   ```

3. Run tests:
   ```bash
   cargo test --release
   ```

## Performance Optimization

- Use ONNX Runtime for CPU efficiency
- Batch multiple extractions together
- Cache model in memory (don't reload per extraction)
- Consider quantized models for faster inference
- Use GPU acceleration if available (CUDA, Metal)

## Next Steps

1. Fine-tune GLiNER on email corpus for better accuracy
2. Add batch processing for multiple emails
3. Implement entity caching/deduplication
4. Add support for different GLiNER model sizes
5. Create model selection based on accuracy/speed tradeoff

## References

- GLiNER paper: https://arxiv.org/abs/2311.08526
- GLiNER GitHub: https://github.com/urchade/GLiNER
- ONNX Runtime: https://onnxruntime.ai/
- Hugging Face model hub: https://huggingface.co/models
