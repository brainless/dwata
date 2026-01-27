# Design and Implement Extraction Framework Foundation

## Objective

Create the foundational extraction framework in the shared-types crate that enables extracting structured entities (Projects, Tasks, Events, Contacts, etc.) from email data using various extraction methods.

This foundation will:
1. Define the core `Extractor` trait that all extractors implement
2. Establish common types for extraction input, output, and configuration
3. Support multiple extraction methods (attachment parsing, pattern-based, NER, LLM)
4. Enable future extensibility for new entity types

## Background

dwata will process emails from IMAP and other sources to automatically extract actionable information:
- **Projects**: Work or hobby projects mentioned in emails
- **Tasks**: Action items, to-dos, assignments
- **Events**: Meetings, deadlines, calendar events
- **Contacts**: People mentioned in emails
- **Locations**: Physical or virtual meeting places
- **Future entities**: Companies, Banks, BankAccounts, etc.

The extraction framework must:
- Support multiple extraction technologies (GLiNER, BERT, LLMs, rule-based)
- Provide confidence scoring for accuracy assessment
- Track provenance (where in the email was this extracted)
- Handle ambiguity (e.g., is this date a project deadline or task due_date?)
- Enable relationship extraction (task belongs to project, event links to task)

## Design Principles

1. **Data Type-First Architecture**: Specialized extractors per entity type for better accuracy
2. **Expandable Entity Types**: Easy to add new entity types in the future
3. **Confidence Scoring**: Both entity-level and field-level confidence
4. **Provenance Tracking**: Record where in email each entity was extracted
5. **Relationship Support**: Link entities together (task→project, event→task)
6. **Multi-Method Support**: Different extraction technologies for different scenarios

## Implementation Plan

### Phase 1: Core Trait Definition

**File: `shared-types/src/extraction.rs`**

Create the core extraction module:

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use ts_rs::TS;

/// Core trait that all extractors must implement
pub trait Extractor {
    /// Extract entities from input
    fn extract(&self, input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError>;

    /// What data type does this extractor target?
    fn data_type(&self) -> DataType;

    /// What extraction method does this extractor use?
    fn method(&self) -> ExtractionMethod;

    /// Get extractor version for tracking
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
}

/// Extraction error types
#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Model error: {0}")]
    ModelError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
```

### Phase 2: Data Type Definitions

```rust
/// Types of entities that can be extracted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum DataType {
    Project,
    Task,
    Event,
    Contact,
    Location,
    Date,        // Standalone dates needing context
    Priority,    // Task priority indicators
    Status,      // Project/task status
    // Future: Company, Bank, BankAccount, etc.
}

/// Extraction methods available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractionMethod {
    AttachmentParsing,  // ICS, VCF, PDF parsing
    PatternBased,       // Regex and keyword matching
    GlinerNER,          // GLiNER named entity recognition
    BertNER,            // BERT-based NER
    LLMBased,           // LLM reasoning for complex cases
    Hybrid,             // Combination of methods
}
```

### Phase 3: Input Types

```rust
/// Input provided to extractors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionInput {
    // Email identification
    pub email_id: String,

    // Email content
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub attachments: Vec<Attachment>,

    // Email metadata
    pub sender: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub timestamp: DateTime<Utc>,
    pub thread_id: Option<String>,
    pub in_reply_to: Option<String>,

    // Context from previous extractions (for chaining)
    pub extracted_entities: Vec<ExtractedEntity>,

    // Context from database
    pub existing_projects: Vec<crate::Project>,
    pub existing_tasks: Vec<crate::Task>,
    pub existing_contacts: Vec<Contact>,

    // User context
    pub user_timezone: String,
    pub user_language: String,
    pub user_preferences: UserPreferences,

    // Extraction control
    pub target_data_type: DataType,
    pub min_confidence: f32,
    pub max_results: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserPreferences {
    pub date_format: String,
    pub default_task_priority: crate::TaskPriority,
    pub default_project_status: crate::ProjectStatus,
    pub auto_link_threshold: f32,  // Confidence for auto-linking entities
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}
```

### Phase 4: Output Types

```rust
/// Result from an extraction
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractionResult {
    // What was extracted
    pub entity: ExtractedEntity,
    pub data_type: DataType,

    // How confident are we?
    pub confidence: f32,
    pub confidence_breakdown: HashMap<String, f32>,  // Per-field confidence

    // How was it extracted?
    pub method: ExtractionMethod,
    pub evidence: Vec<TextSpan>,  // Where in email

    // Relationships to other entities
    pub relationships: Vec<Relationship>,

    // Should user review this?
    pub requires_review: bool,
    pub ambiguities: Vec<Ambiguity>,

    // Provenance
    pub extracted_at: DateTime<Utc>,
    pub extractor_version: String,
}

/// Extracted entity (can be any type)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "data")]
pub enum ExtractedEntity {
    Project(ExtractedProject),
    Task(ExtractedTask),
    Event(ExtractedEvent),
    Contact(ExtractedContact),
    Location(ExtractedLocation),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractedProject {
    pub name: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub status: Option<crate::ProjectStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractedTask {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<crate::TaskPriority>,
    pub due_date: Option<String>,
    pub assigned_to: Option<String>,
    pub project_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractedEvent {
    pub name: String,
    pub description: Option<String>,
    pub date: String,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub project_id: Option<i32>,
    pub task_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractedContact {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractedLocation {
    pub name: String,
    pub address: Option<String>,
    pub coordinates: Option<(f64, f64)>,
}
```

### Phase 5: Supporting Types

```rust
/// Location of text in email
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TextSpan {
    pub source: TextSource,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "data")]
pub enum TextSource {
    Subject,
    Body,
    Attachment(String),  // filename
}

/// Relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Relationship {
    pub relation_type: RelationType,
    pub target_entity: EntityRef,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum RelationType {
    BelongsToProject,    // Task -> Project
    LinkedToTask,        // Event -> Task
    AssignedTo,          // Task -> Contact
    LocatedAt,           // Event -> Location
    HasDeadline,         // Project/Task -> Date
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntityRef {
    pub data_type: DataType,
    pub entity_id: Option<i32>,  // If already in DB
    pub extracted_index: Option<usize>,  // Index in current extraction batch
}

/// Ambiguity in extraction
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Ambiguity {
    pub field: String,
    pub options: Vec<AmbiguityOption>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AmbiguityOption {
    pub value: String,
    pub confidence: f32,
}
```

### Phase 6: Update shared-types lib.rs

**File: `shared-types/src/lib.rs`**

Add extraction module:

```rust
use serde::{Deserialize, Serialize};

pub mod event;
pub mod extraction;  // NEW
pub mod project;
pub mod session;
pub mod settings;
pub mod task;

// Re-export extraction types
pub use extraction::*;

// ... rest of existing exports
```

### Phase 7: Update Cargo.toml

**File: `shared-types/Cargo.toml`**

Add dependencies:

```toml
[dependencies]
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
ts-rs = "7.0"
thiserror = "1.0"
```

## Testing

### Unit Tests

**File: `shared-types/src/extraction.rs`**

Add tests at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_serialization() {
        let dt = DataType::Project;
        let json = serde_json::to_string(&dt).unwrap();
        assert_eq!(json, "\"project\"");

        let deserialized: DataType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, dt);
    }

    #[test]
    fn test_extraction_method_serialization() {
        let method = ExtractionMethod::GlinerNER;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, "\"gliner-ner\"");
    }

    #[test]
    fn test_extracted_entity_serialization() {
        let entity = ExtractedEntity::Task(ExtractedTask {
            title: "Complete homepage mockup".to_string(),
            description: Some("Design task".to_string()),
            priority: Some(crate::TaskPriority::High),
            due_date: Some("2026-01-25".to_string()),
            assigned_to: Some("Sarah".to_string()),
            project_id: Some(1),
        });

        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: ExtractedEntity = serde_json::from_str(&json).unwrap();

        match deserialized {
            ExtractedEntity::Task(task) => {
                assert_eq!(task.title, "Complete homepage mockup");
            }
            _ => panic!("Wrong entity type"),
        }
    }

    #[test]
    fn test_extraction_result_creation() {
        let result = ExtractionResult {
            entity: ExtractedEntity::Project(ExtractedProject {
                name: "Website Redesign".to_string(),
                description: Some("Redesign company website".to_string()),
                deadline: Some("2026-02-15".to_string()),
                status: Some(crate::ProjectStatus::Active),
            }),
            data_type: DataType::Project,
            confidence: 0.85,
            confidence_breakdown: HashMap::new(),
            method: ExtractionMethod::PatternBased,
            evidence: vec![],
            relationships: vec![],
            requires_review: false,
            ambiguities: vec![],
            extracted_at: Utc::now(),
            extractor_version: "1.0.0".to_string(),
        };

        assert_eq!(result.confidence, 0.85);
        assert_eq!(result.data_type, DataType::Project);
    }
}
```

### Build and Test

```bash
# Build shared-types
cd shared-types
cargo build

# Run tests
cargo test

# Generate TypeScript bindings
cargo test  # ts-rs exports during test phase
```

### Verify TypeScript Bindings

Check that TypeScript types are generated:

```bash
ls -la gui/src/api-types/
# Should see extraction-related .ts files
```

## Next Steps

After completing this foundation:

1. **Create extractors crate**: New workspace member for extraction implementations
2. **Implement Attachment Parser**: Task #2 - Parse ICS and VCF files
3. **Implement Pattern-Based Extractor**: Task #3 - Regex and keyword matching
4. **Implement GLiNER Extractor**: Task #4 - NER-based extraction
5. **Implement LLM Extractor**: Task #5 - LLM reasoning for complex cases

## References

- GLiNER: https://github.com/urchade/GLiNER
- ONNX Runtime: https://github.com/pykeio/ort
- ts-rs for TypeScript bindings: https://github.com/Aleph-Alpha/ts-rs
