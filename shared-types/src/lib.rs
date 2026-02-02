use serde::{Deserialize, Serialize};

pub mod company;
pub mod contact;
pub mod contact_link;
pub mod credential;
pub mod download;
pub mod email;
pub mod event;
pub mod extraction;
pub mod extraction_job;
pub mod financial;
pub mod position;
pub mod project;
pub mod session;
pub mod settings;
pub mod task;

pub use company::{CompaniesResponse, Company, CreateCompanyRequest, UpdateCompanyRequest};
pub use contact::{Contact, ContactsResponse, CreateContactRequest, UpdateContactRequest};
pub use contact_link::{
    ContactLink, ContactLinkType, ContactLinksResponse, CreateContactLinkRequest,
};
pub use credential::{
    ApiKeySettings, CreateCredentialRequest, CreateImapCredentialRequest, CredentialListResponse,
    CredentialMetadata, CredentialType, ImapAccountSettings, ImapAuthMethod,
    ImapCredentialMetadata, PasswordResponse, SmtpAccountSettings, UpdateCredentialRequest,
};
pub use download::{
    CloudStorageDownloadState, CreateDownloadJobRequest, DirectoryStatus, DownloadItem,
    DownloadItemStatus, DownloadJob, DownloadJobListResponse, DownloadJobStatus, DownloadProgress,
    FileFilter, ImapDownloadState, ImapFolderStatus, ImapSyncStrategy, SourceType,
};
pub use email::{
    AttachmentExtractionStatus, Email, EmailAddress, EmailAttachment, ListEmailsRequest,
    ListEmailsResponse,
};
pub use event::{CreateEventRequest, Event, EventsResponse, UpdateEventRequest};
pub use extraction_job::{
    ArchiveType, AttachmentExtractionFilter, CreateExtractionJobRequest, ExtractionJob,
    ExtractionJobListResponse, ExtractionJobStatus, ExtractionProgress, ExtractionSourceConfig,
    ExtractionSourceType, ExtractorType,
};
pub use financial::{
    CategoryBreakdown, FinancialDocumentType, FinancialHealth, FinancialPattern, FinancialSummary,
    FinancialTransaction, TransactionCategory, TransactionStatus,
};
pub use position::{CreatePositionRequest, Position, PositionsResponse};
pub use project::{
    CreateProjectRequest, Project, ProjectStatus, ProjectsResponse, UpdateProjectRequest,
};
pub use session::{
    AgentMessage, AgentSession, AgentToolCall, SessionListItem, SessionListResponse,
    SessionMessage, SessionResponse, SessionToolCall,
};
pub use settings::{ApiKeyConfig, SettingsResponse, UpdateApiKeysRequest};
pub use task::{
    CreateTaskRequest, Task, TaskPriority, TaskStatus, TasksResponse, UpdateTaskRequest,
};

// Re-export extraction types
pub use extraction::*;

/// Error response for API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Request to create a new agent session
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_name: String,
    pub user_prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub config: Option<serde_json::Value>,
}

/// Response after creating a session
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: i64,
    pub agent_name: String,
    pub status: String,
}
