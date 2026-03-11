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
pub mod financial;
pub mod financial_template;
pub mod folder;
pub mod label;
pub mod location;
pub mod ollama;
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
    AttachmentExtractionStatus, Email, EmailAddress, EmailAttachment, EmailsByIdsRequest,
    EmailsByIdsResponse, ListEmailsRequest, ListEmailsResponse,
};
pub use event::{CreateEventRequest, Event, EventsResponse, UpdateEventRequest};
pub use financial::{
    CategoryBreakdown, FinancialExtractionSummary, FinancialHealth, FinancialPagination,
    FinancialSummary, ListFinancialBillsResponse,
};
pub use financial_template::{
    DeleteFinancialTemplatesRequest, DeleteFinancialTemplatesResponse,
    DetectFinancialTemplatesRequest, DetectFinancialTemplatesResponse, DetectedFinancialTemplate,
    DetectedFinancialTemplateVariable, FinancialExtractionTemplate, FinancialTemplateApplicability,
    FinancialTemplateDetectionJobState, FinancialTemplateDetectionJobStatus,
    FinancialTemplateFieldMapping, FinancialTemplateStatus, FinancialTemplateType,
    FinancialTemplateVariable, FinancialTemplateWithVariables, ListFinancialTemplatesResponse,
    TemplateDetectionDebugState, TemplateDetectionGeneratedTemplateDebug,
    TemplateDetectionSenderDebug, TemplateDetectionSenderLlmDraftPreview,
    TemplateDetectionSenderLlmInputsResponse, TemplateDetectionSenderRank,
};
pub use folder::{EmailFolder, ListFoldersRequest, ListFoldersResponse};
pub use label::{EmailLabel, ListLabelsRequest, ListLabelsResponse};
pub use location::{CreateLocationRequest, Location, LocationsResponse, UpdateLocationRequest};
pub use ollama::{
    OllamaModelDetails, OllamaModelInfo, OllamaModelsResponse, OllamaPullModelRequest,
    OllamaPullModelResponse, OllamaStatusResponse,
};
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
pub use transaction::{DataSourceType, Transaction, TransactionCategory, TransactionStatus};
pub use vendor::{Vendor, VendorType};

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
