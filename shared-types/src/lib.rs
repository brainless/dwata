use serde::{Deserialize, Serialize};

pub mod bill;
pub mod contact_link;
pub mod credential;
pub mod document_label;
pub mod download;
pub mod email;
pub mod entity_types;
pub mod event;
pub mod extraction;
pub mod financial;
pub mod folder;
pub mod kg_extraction;
pub mod kg_pass;
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
pub use contact_link::{
    ContactLink, ContactLinkType, ContactLinksResponse, CreateContactLinkRequest,
};
pub use credential::{
    ApiKeySettings, CreateCredentialRequest, CredentialListResponse, CredentialMetadata,
    CredentialType, ImapAccountSettings, ImapAuthMethod, ImapCredentialMetadata, PasswordResponse,
    SmtpAccountSettings, UpdateCredentialRequest,
};
pub use document_label::{DocumentType, LabelDocumentParams};
pub use download::{EmailSyncDirection, TriggerAllEmailSyncRequest, TriggerEmailSyncRequest};
pub use email::{
    Email, EmailAddress, EmailsByIdsRequest, EmailsByIdsResponse, ListEmailsRequest,
    ListEmailsResponse,
};
pub use entity_types::{
    ConfirmEntitiesParams, ExtractedBill, ExtractedEntitiesParams, ExtractedEvent,
    ExtractedLocation, ExtractedOrder, ExtractedOrganisation, ExtractedPerson,
    ExtractedSubscription, ExtractedTransaction,
};
pub use event::{Event, EventsResponse};
pub use extraction::{
    count_entities_by_type, ExtractionStatus, ExtractionStep, ExtractionStepState,
    ExtractionSummary, PassStatus, PassStepState, RetryReason,
};
pub use financial::{
    CategoryBreakdown, FinancialExtractionSummary, FinancialPagination, ListFinancialBillsResponse,
};
pub use folder::{EmailFolder, ListFoldersRequest, ListFoldersResponse};
pub use kg_extraction::{EntitySearchResult, NamedEntityKind, SearchEntitiesParams};
pub use kg_pass::KgPassType;
pub use label::{EmailLabel, ListLabelsRequest, ListLabelsResponse};
pub use location::{Location, LocationsResponse};
pub use ollama::{
    OllamaModelDetails, OllamaModelInfo, OllamaModelsResponse, OllamaPullModelRequest,
    OllamaPullModelResponse, OllamaStatusResponse,
};
pub use order::{Order, OrderItem, OrderStatus, OrdersResponse};

pub use organisation::{
    CreateOrganisationRequest, Organisation, OrganisationRole, OrganisationWithCounts,
    OrganisationsResponse, OrganisationsWithCountsResponse, UpdateOrganisationRequest,
};
pub use person::{Person, PersonWithCounts, PersonsResponse, PersonsWithCountsResponse};
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
