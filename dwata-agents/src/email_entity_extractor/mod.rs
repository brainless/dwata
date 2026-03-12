pub mod agent;
pub mod prompts;
pub mod search;
pub mod types;

pub use agent::EmailEntityExtractorAgent;
pub use search::{EmailSearchProvider, EmailSearchResult, SearchEmailsParams};
pub use types::{
    parse_value, ConfirmEntitiesParams, ExtractedBill, ExtractedEntitiesParams, ExtractedEvent,
    ExtractedLocation, ExtractedOrder, ExtractedOrganisation, ExtractedPerson,
    ExtractedSubscription, ExtractedTransaction, ParsedValue,
};
