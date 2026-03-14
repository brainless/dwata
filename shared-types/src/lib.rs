use serde::{Deserialize, Serialize};

pub mod bill;
pub mod contact;
pub mod contact_link;
pub mod credential;
pub mod download;
pub mod email;
pub mod event;
pub mod financial;
pub mod financial_template;
pub mod folder;
pub mod label;
pub mod location;
pub mod ollama;
pub mod order;
pub mod organisation;
pub mod person;
pub mod project;
pub mod search;
pub mod session;
pub mod settings;
pub mod subscription;
pub mod task;
pub mod transaction;
pub use bill::{Bill, BillStatus, BillSubject, ServiceIdentifierKind};
pub use contact::{Contact, ContactsResponse};
pub use contact_link::{
    ContactLink, ContactLinkType, ContactLinksResponse, CreateContactLinkRequest,
};
pub use credential::{
    ApiKeySettings, CreateCredentialRequest, CredentialListResponse, CredentialMetadata,
    CredentialType, ImapAccountSettings, ImapAuthMethod, ImapCredentialMetadata, PasswordResponse,
    SmtpAccountSettings, UpdateCredentialRequest,
};

pub use download::{
    EmailSyncDirection, EmailSyncSettings, PauseEmailSyncRequest, ResumeEmailSyncRequest,
    TriggerAllEmailSyncRequest, TriggerEmailSyncRequest,
};
pub use email::{
    Email, EmailAddress, EmailsByIdsRequest, EmailsByIdsResponse, ListEmailsRequest,
    ListEmailsResponse,
};
pub use event::{Event, EventsResponse};
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
pub use location::Location;
pub use ollama::{
    OllamaModelDetails, OllamaModelInfo, OllamaModelsResponse, OllamaPullModelRequest,
    OllamaPullModelResponse, OllamaStatusResponse,
};
pub use order::{Order, OrderStatus, OrdersResponse};

pub use organisation::{
    CreateOrganisationRequest, Organisation, OrganisationRole, OrganisationsResponse,
    UpdateOrganisationRequest,
};
pub use person::{Person, PersonsResponse};
pub use project::{Project, ProjectStatus};
pub use search::{
    HitId, SearchField, SearchFieldLegacy, SearchHit, SearchHitLegacy, SearchRequest,
    SearchResponse, SearchTarget, SearchTerm, SearchTermLegacy,
};
pub use session::{
    AgentMessage, AgentSession, AgentToolCall, SessionListItem, SessionMessage, SessionResponse,
    SessionToolCall,
};
pub use settings::{
    AiProviderApiKeyConfig, OAuthClientAppConfig, SettingsResponse, UpdateAiProviderApiKeysRequest,
    UpdateOAuthClientAppsRequest,
};
pub use subscription::{BillingCycle, Subscription, SubscriptionsResponse};

pub use task::{Task, TaskPriority, TaskStatus};
pub use transaction::{DataSourceType, Transaction, TransactionCategory, TransactionStatus};

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
