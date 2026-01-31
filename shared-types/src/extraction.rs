use serde::{Deserialize, Serialize};
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

/// Types of entities that can be extracted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum DataType {
    Project,
    Task,
    Event,
    Contact,
    Location,
    Date,
    Priority,
    Status,
    Company,
    Position,
}

/// Extraction methods available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractionMethod {
    AttachmentParsing,
    PatternBased,
    GlinerNER,
    BertNER,
    LLMBased,
    Hybrid,
}

/// Input provided to extractors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
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
    pub timestamp: i64,
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
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UserPreferences {
    pub date_format: String,
    pub default_task_priority: crate::TaskPriority,
    pub default_project_status: crate::ProjectStatus,
    pub auto_link_threshold: f32, // Confidence for auto-linking entities
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

/// Result from an extraction
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractionResult {
    // What was extracted
    pub entity: ExtractedEntity,
    pub data_type: DataType,

    // How confident are we?
    pub confidence: f32,
    pub confidence_breakdown: HashMap<String, f32>, // Per-field confidence

    // How was it extracted?
    pub method: ExtractionMethod,
    pub evidence: Vec<TextSpan>, // Where in email

    // Relationships to other entities
    pub relationships: Vec<Relationship>,

    // Should user review this?
    pub requires_review: bool,
    pub ambiguities: Vec<Ambiguity>,

    // Provenance
    pub extracted_at: i64,
    pub extractor_version: String,
}

/// Extracted entity (can be any type)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data")]
pub enum ExtractedEntity {
    Project(ExtractedProject),
    Task(ExtractedTask),
    Event(ExtractedEvent),
    Contact(ExtractedContact),
    Location(ExtractedLocation),
    Company(ExtractedCompany),
    Position(ExtractedPosition),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedProject {
    pub name: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub status: Option<crate::ProjectStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedTask {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<crate::TaskPriority>,
    pub due_date: Option<String>,
    pub assigned_to: Option<String>,
    pub project_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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
pub struct ExtractedContact {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    #[serde(default)]
    pub profile_urls: Vec<ProfileUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedLocation {
    pub name: String,
    pub address: Option<String>,
    pub coordinates: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProfileUrl {
    pub url: String,
    pub link_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedCompany {
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedPosition {
    pub contact_name: String,
    pub company_name: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub started_on: Option<String>,
    pub finished_on: Option<String>,
    pub is_current: bool,
}

/// Location of text in email
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TextSpan {
    pub source: TextSource,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data")]
pub enum TextSource {
    Subject,
    Body,
    Attachment(String), // filename
}

/// Relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Relationship {
    pub relation_type: RelationType,
    pub target_entity: EntityRef,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum RelationType {
    BelongsToProject, // Task -> Project
    LinkedToTask,     // Event -> Task
    AssignedTo,       // Task -> Contact
    LocatedAt,        // Event -> Location
    HasDeadline,      // Project/Task -> Date
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EntityRef {
    pub data_type: DataType,
    pub entity_id: Option<i32>,         // If already in DB
    pub extracted_index: Option<usize>, // Index in current extraction batch
}

/// Ambiguity in extraction
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Ambiguity {
    pub field: String,
    pub options: Vec<AmbiguityOption>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AmbiguityOption {
    pub value: String,
    pub confidence: f32,
}

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
        assert_eq!(json, "\"gliner-n-e-r\"");
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
            extracted_at: chrono::Utc::now().timestamp(),
            extractor_version: "1.0.0".to_string(),
        };

        assert_eq!(result.confidence, 0.85);
        assert_eq!(result.data_type, DataType::Project);
    }
}
