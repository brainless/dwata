use shared_types::credential::LocalFileSettings;
use shared_types::*;
use std::fs;
use std::path::Path;
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate TypeScript definitions for API types
    let mut types = Vec::new();

    // Project types
    types.push(clean_type(Project::export_to_string()?));
    types.push(clean_type(ProjectStatus::export_to_string()?));

    // Event types
    types.push(clean_type(Event::export_to_string()?));
    types.push(clean_type(EventsResponse::export_to_string()?));

    // Task types
    types.push(clean_type(Task::export_to_string()?));
    types.push(clean_type(TaskStatus::export_to_string()?));
    types.push(clean_type(TaskPriority::export_to_string()?));

    // Session types
    types.push(clean_type(SessionMessage::export_to_string()?));
    types.push(clean_type(SessionToolCall::export_to_string()?));
    types.push(clean_type(SessionResponse::export_to_string()?));
    types.push(clean_type(SessionListItem::export_to_string()?));

    // Settings types
    types.push(clean_type(AiProviderApiKeyConfig::export_to_string()?));
    types.push(clean_type(OAuthClientAppConfig::export_to_string()?));
    types.push(clean_type(SettingsResponse::export_to_string()?));
    types.push(clean_type(
        UpdateAiProviderApiKeysRequest::export_to_string()?,
    ));
    types.push(clean_type(UpdateOAuthClientAppsRequest::export_to_string()?));

    // Credential types
    types.push(clean_type(CredentialType::export_to_string()?));
    types.push(clean_type(CreateCredentialRequest::export_to_string()?));
    types.push(clean_type(UpdateCredentialRequest::export_to_string()?));
    types.push(clean_type(CredentialMetadata::export_to_string()?));
    types.push(clean_type(PasswordResponse::export_to_string()?));
    types.push(clean_type(CredentialListResponse::export_to_string()?));

    // IMAP credential types
    types.push(clean_type(ImapAuthMethod::export_to_string()?));
    types.push(clean_type(ImapAccountSettings::export_to_string()?));
    types.push(clean_type(ImapCredentialMetadata::export_to_string()?));

    // SMTP credential types
    types.push(clean_type(SmtpAccountSettings::export_to_string()?));

    // API Key credential types
    types.push(clean_type(ApiKeySettings::export_to_string()?));

    // Local File credential types
    types.push(clean_type(LocalFileSettings::export_to_string()?));

    // Email sync types
    types.push(clean_type(EmailSyncDirection::export_to_string()?));
    types.push(clean_type(TriggerEmailSyncRequest::export_to_string()?));

    // Email types
    types.push(clean_type(Email::export_to_string()?));
    types.push(clean_type(EmailAddress::export_to_string()?));
    types.push(clean_type(ListEmailsRequest::export_to_string()?));
    types.push(clean_type(ListEmailsResponse::export_to_string()?));
    types.push(clean_type(EmailsByIdsRequest::export_to_string()?));
    types.push(clean_type(EmailsByIdsResponse::export_to_string()?));

    // Email Folder and Label types
    types.push(clean_type(EmailFolder::export_to_string()?));
    types.push(clean_type(ListFoldersRequest::export_to_string()?));
    types.push(clean_type(ListFoldersResponse::export_to_string()?));
    types.push(clean_type(EmailLabel::export_to_string()?));
    types.push(clean_type(ListLabelsRequest::export_to_string()?));
    types.push(clean_type(ListLabelsResponse::export_to_string()?));

    // Location types
    types.push(clean_type(Location::export_to_string()?));
    types.push(clean_type(LocationsResponse::export_to_string()?));

    // Person types
    types.push(clean_type(Person::export_to_string()?));
    types.push(clean_type(PersonsResponse::export_to_string()?));

    // Financial types
    types.push(clean_type(DataSourceType::export_to_string()?));
    types.push(clean_type(Bill::export_to_string()?));
    types.push(clean_type(BillStatus::export_to_string()?));
    types.push(clean_type(BillSubject::export_to_string()?));
    types.push(clean_type(ServiceIdentifierKind::export_to_string()?));
    types.push(clean_type(Transaction::export_to_string()?));
    types.push(clean_type(TransactionCategory::export_to_string()?));
    types.push(clean_type(TransactionStatus::export_to_string()?));
    types.push(clean_type(Subscription::export_to_string()?));
    types.push(clean_type(BillingCycle::export_to_string()?));
    types.push(clean_type(SubscriptionsResponse::export_to_string()?));
    types.push(clean_type(Order::export_to_string()?));
    types.push(clean_type(OrderItem::export_to_string()?));
    types.push(clean_type(OrderStatus::export_to_string()?));
    types.push(clean_type(OrdersResponse::export_to_string()?));
    types.push(clean_type(FinancialExtractionSummary::export_to_string()?));
    types.push(clean_type(CategoryBreakdown::export_to_string()?));
    types.push(clean_type(FinancialPagination::export_to_string()?));
    types.push(clean_type(ListFinancialBillsResponse::export_to_string()?));

    let output_dir = Path::new("../gui/src/api-types");
    fs::create_dir_all(output_dir)?;

    let output_path = output_dir.join("types.ts");
    let output = types.join("\n\n");

    fs::write(&output_path, output)?;
    println!("Generated TypeScript types in {}", output_path.display());

    Ok(())
}

fn clean_type(mut type_def: String) -> String {
    type_def.retain(|c| c != '\r');

    // Check if the type definition includes imports (like Email which imports EmailAddress)
    let lines: Vec<&str> = type_def.lines().collect();
    let has_import = lines
        .iter()
        .any(|line| line.trim().starts_with("import type"));

    let filtered: Vec<&str> = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            // Keep import lines if they're part of a type definition (Email type imports EmailAddress)
            if trimmed.starts_with("import type") {
                return has_import;
            }
            // Filter out the generated comment line
            !trimmed.starts_with("// This file was generated")
                && !trimmed.starts_with("/* This file was generated")
        })
        .cloned()
        .collect();

    let result = filtered.join("\n").trim().to_string();
    if result.is_empty() {
        result
    } else {
        format!("{}\n", result)
    }
}
