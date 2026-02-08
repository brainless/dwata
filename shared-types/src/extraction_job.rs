use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Extraction job for processing attachments and extracting entities
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractionJob {
    pub id: i64,
    pub source_type: ExtractionSourceType,
    pub extractor_type: ExtractorType,
    pub status: ExtractionJobStatus,
    pub progress: ExtractionProgress,
    #[ts(skip)]
    pub source_config: serde_json::Value,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractionSourceType {
    EmailAttachment,
    LocalFile,
    LocalArchive,
    EmailBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractorType {
    AttachmentParser,
    LinkedInArchive,
    GlinerNER,
    LLMBased,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractionJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractionProgress {
    pub total_items: u64,
    pub processed_items: u64,
    pub extracted_entities: u64,
    pub failed_items: u64,
    pub events_extracted: u64,
    pub contacts_extracted: u64,
    pub companies_extracted: u64,
    pub positions_extracted: u64,
    pub percent_complete: f32,
}

/// Configuration for extraction source
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "config")]
pub enum ExtractionSourceConfig {
    EmailAttachments {
        email_ids: Option<Vec<i64>>,
        attachment_types: Vec<String>,
        status_filter: AttachmentExtractionFilter,
    },
    LocalFile {
        file_path: String,
        content_type: String,
    },
    LocalArchive {
        archive_path: String,
        archive_type: ArchiveType,
        files_to_process: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveType {
    LinkedIn,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum AttachmentExtractionFilter {
    Pending,          // Only process pending attachments
    PendingAndFailed, // Retry failed ones
    All,              // Reprocess everything
}

/// Request to create extraction job
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateExtractionJobRequest {
    pub source_type: ExtractionSourceType,
    pub extractor_type: ExtractorType,
    pub source_config: ExtractionSourceConfig,
}

/// Response for extraction job list
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ExtractionJobListResponse {
    pub jobs: Vec<ExtractionJob>,
}
