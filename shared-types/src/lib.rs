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
pub mod search;
pub mod session;
pub mod settings;
pub mod subscription;
pub mod transaction;

pub use bill::{Bill, BillStatus};
pub use contact_link::{ContactLink, ContactLinkType, ContactLinksResponse};
pub use credential::{
    ApiKeySettings, CreateCredentialRequest, CredentialListResponse, CredentialMetadata,
    CredentialType, ImapAccountSettings, ImapAuthMethod, PasswordResponse, SmtpAccountSettings,
    UpdateCredentialRequest,
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
pub use financial::{FinancialPagination, ListFinancialBillsResponse};
pub use folder::{EmailFolder, ListFoldersResponse};
pub use kg_extraction::{EntitySearchResult, NamedEntityKind, SearchEntitiesParams};
pub use kg_pass::KgPassType;
pub use label::{EmailLabel, ListLabelsResponse};
pub use location::{Location, LocationsResponse};
pub use ollama::{
    OllamaModelDetails, OllamaModelInfo, OllamaModelsResponse, OllamaPullModelRequest,
    OllamaPullModelResponse, OllamaStatusResponse,
};
pub use order::{Order, OrderItem, OrderStatus, OrdersResponse};

pub use organisation::{
    Organisation, OrganisationRole, OrganisationWithCounts, OrganisationsWithCountsResponse,
};
pub use person::{Person, PersonWithCounts, PersonsWithCountsResponse};
pub use search::{
    HitId, SearchField, SearchHit, SearchRequest, SearchResponse, SearchTarget, SearchTerm,
};
pub use session::{AgentMessage, AgentSession, AgentToolCall, SessionListItem};
pub use settings::{
    AiProviderApiKeyConfig, OAuthClientAppConfig, SettingsResponse, UpdateAiProviderApiKeysRequest,
    UpdateOAuthClientAppsRequest,
};
pub use subscription::{BillingCycle, Subscription, SubscriptionsResponse};

pub use transaction::{DataSourceType, Transaction, TransactionCategory, TransactionStatus};

/// Error response for API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
