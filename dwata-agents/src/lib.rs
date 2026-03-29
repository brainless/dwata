pub mod date_parser;
pub mod email_entity_extractor;
pub mod entity_search;
pub mod entity_type_manifest;
pub mod kg_email_extractor;
pub mod kg_pass_context;
pub mod kg_persistence;
pub mod llm_template_variable_extractor;
pub mod statement_extractor;
pub mod storage;
pub mod template_detection;
pub mod template_document_labeler;
pub mod template_value_extractor;

pub use email_entity_extractor::{EmailEntityExtractorAgent, ExtractedEntitiesParams};
pub use entity_search::{
    EntitySearchProvider, EntitySearchResult, NamedEntityKind, SearchEntitiesParams,
};
pub use entity_type_manifest::{existing_entities_section, generate_entity_manifest};
pub use kg_email_extractor::KgEmailExtractionAgent;
pub use kg_pass_context::{KgExtractionPass, KgPassType};
pub use kg_persistence::KgPersistenceProvider;
pub use llm_template_variable_extractor::{
    LlmTemplateVariableExtractorAgent, TemplateVariable, TemplateVariableParams,
    TemplateVariableType,
};
pub use statement_extractor::{ColumnarSheet, StatementField, StatementTemplate};
pub use storage::{AgentStorage, Message, Session, ToolCall};
pub use template_detection::{
    detect_reverse_templates_for_sender, detect_templates_for_sender, discover_template_drafts,
    reconstruct_template_from_variables, simple_email_content, DetectedTemplateCluster,
    NormalizedEmailContent, TemplateDetectionOptions, TemplateDraft, TemplateInputEmail,
    TemplateVariableMapping,
};
pub use template_detection::{
    ReverseTemplateField, ReverseTemplateType, ReverseVariableTranslation,
};
pub use template_document_labeler::{LabelDocumentParams, TemplateDocumentLabelerAgent};
pub use template_value_extractor::{
    extract_values_from_email_with_values, extract_values_using_template, parse_amount, parse_date,
    TemplateEmailContent,
};
