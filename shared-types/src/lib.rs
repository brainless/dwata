use serde::{Deserialize, Serialize};

pub mod bill;
pub mod company;
pub mod contact;
pub mod contact_link;
pub mod credential;
pub mod document;
pub mod download;
pub mod email;
pub mod event;
pub mod extraction;
pub mod extraction_job;
pub mod financial;
pub mod folder;
pub mod label;
pub mod position;
pub mod project;
pub mod search;
pub mod session;
pub mod settings;
pub mod task;
pub mod transaction;
pub mod vendor;

pub use bill::{Bill, BillStatus, BillSubject, FinancialDocumentType, ServiceIdentifierKind};
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
pub use document::{
    Document, DocumentCursor, DocumentKind, DocumentSortBy, DocumentSource, DocumentSourceType,
    ListDocumentsRequest, ListDocumentsResponse, SourceAccessState, SourcePermissionState,
};
pub use download::{
    CloudStorageDownloadState, CreateDownloadJobRequest, DirectoryStatus, DownloadItem,
    DownloadItemStatus, DownloadJob, DownloadJobListResponse, DownloadJobStatus, DownloadProgress,
    FileFilter, ImapDownloadState, ImapFolderStatus, ImapSyncStrategy, SourceType,
};
pub use email::{
    AttachmentExtractionStatus, Email, EmailAddress, EmailAttachment, FinancialEmailScanRequest,
    FinancialEmailScanResponse, FinancialEmailScanSender, ListEmailsRequest, ListEmailsResponse,
};
pub use event::{CreateEventRequest, Event, EventsResponse, UpdateEventRequest};
pub use extraction_job::{
    ArchiveType, AttachmentExtractionFilter, CreateExtractionJobRequest, ExtractionJob,
    ExtractionJobListResponse, ExtractionJobStatus, ExtractionProgress, ExtractionSourceConfig,
    ExtractionSourceType, ExtractorType,
};
pub use financial::{
    CategoryBreakdown, FinancialExtractionSummary, FinancialHealth, FinancialSummary,
};
pub use folder::{EmailFolder, ListFoldersRequest, ListFoldersResponse};
pub use label::{EmailLabel, ListLabelsRequest, ListLabelsResponse};
pub use position::{CreatePositionRequest, Position, PositionsResponse};
pub use project::{
    CreateProjectRequest, Project, ProjectStatus, ProjectsResponse, UpdateProjectRequest,
};
pub use search::{
    SearchDocumentsRequest, SearchDocumentsResponse, SearchField, SearchHit, SearchTerm,
};
pub use session::{
    AgentMessage, AgentSession, AgentToolCall, SessionListItem, SessionListResponse,
    SessionMessage, SessionResponse, SessionToolCall,
};
pub use settings::{
    AiProviderApiKeyConfig, OAuthClientAppConfig, SettingsResponse, UpdateAiProviderApiKeysRequest,
    UpdateOAuthClientAppsRequest,
};
pub use task::{
    CreateTaskRequest, Task, TaskPriority, TaskStatus, TasksResponse, UpdateTaskRequest,
};
pub use transaction::{
    DataSourceType, EnrichmentStatus, FinancialTransaction, TransactionCategory, TransactionParty,
    TransactionStatus, UnresolvedField,
};
pub use vendor::{TransactionVendor, TransactionVendorType};

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
