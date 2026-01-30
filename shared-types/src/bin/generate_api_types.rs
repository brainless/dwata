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
    types.push(clean_type(CreateProjectRequest::export_to_string()?));
    types.push(clean_type(UpdateProjectRequest::export_to_string()?));
    types.push(clean_type(ProjectsResponse::export_to_string()?));

    // Event types
    types.push(clean_type(Event::export_to_string()?));
    types.push(clean_type(CreateEventRequest::export_to_string()?));
    types.push(clean_type(UpdateEventRequest::export_to_string()?));
    types.push(clean_type(EventsResponse::export_to_string()?));

    // Task types
    types.push(clean_type(Task::export_to_string()?));
    types.push(clean_type(TaskStatus::export_to_string()?));
    types.push(clean_type(TaskPriority::export_to_string()?));
    types.push(clean_type(CreateTaskRequest::export_to_string()?));
    types.push(clean_type(UpdateTaskRequest::export_to_string()?));
    types.push(clean_type(TasksResponse::export_to_string()?));

    // Session types
    types.push(clean_type(SessionMessage::export_to_string()?));
    types.push(clean_type(SessionToolCall::export_to_string()?));
    types.push(clean_type(SessionResponse::export_to_string()?));
    types.push(clean_type(SessionListItem::export_to_string()?));
    types.push(clean_type(SessionListResponse::export_to_string()?));

    // Settings types
    types.push(clean_type(ApiKeyConfig::export_to_string()?));
    types.push(clean_type(SettingsResponse::export_to_string()?));
    types.push(clean_type(UpdateApiKeysRequest::export_to_string()?));

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
    types.push(clean_type(CreateImapCredentialRequest::export_to_string()?));
    types.push(clean_type(ImapCredentialMetadata::export_to_string()?));

    // SMTP credential types
    types.push(clean_type(SmtpAccountSettings::export_to_string()?));

    // API Key credential types
    types.push(clean_type(ApiKeySettings::export_to_string()?));

    // Download types
    types.push(clean_type(DownloadJob::export_to_string()?));
    types.push(clean_type(DownloadJobStatus::export_to_string()?));
    types.push(clean_type(DownloadProgress::export_to_string()?));
    types.push(clean_type(SourceType::export_to_string()?));
    types.push(clean_type(ImapDownloadState::export_to_string()?));
    types.push(clean_type(ImapFolderStatus::export_to_string()?));
    types.push(clean_type(ImapSyncStrategy::export_to_string()?));
    types.push(clean_type(CloudStorageDownloadState::export_to_string()?));
    types.push(clean_type(DirectoryStatus::export_to_string()?));
    types.push(clean_type(FileFilter::export_to_string()?));
    types.push(clean_type(CreateDownloadJobRequest::export_to_string()?));
    types.push(clean_type(DownloadJobListResponse::export_to_string()?));
    types.push(clean_type(DownloadItem::export_to_string()?));
    types.push(clean_type(DownloadItemStatus::export_to_string()?));

    // Email types
    types.push(clean_type(Email::export_to_string()?));
    types.push(clean_type(EmailAddress::export_to_string()?));
    types.push(clean_type(EmailAttachment::export_to_string()?));
    types.push(clean_type(AttachmentExtractionStatus::export_to_string()?));
    types.push(clean_type(ListEmailsRequest::export_to_string()?));
    types.push(clean_type(ListEmailsResponse::export_to_string()?));

    // Extraction types
    types.push(clean_type(DataType::export_to_string()?));
    types.push(clean_type(ExtractionMethod::export_to_string()?));
    types.push(clean_type(Attachment::export_to_string()?));
    types.push(clean_type(UserPreferences::export_to_string()?));
    types.push(clean_type(ExtractedEntity::export_to_string()?));
    types.push(clean_type(ExtractedProject::export_to_string()?));
    types.push(clean_type(ExtractedTask::export_to_string()?));
    types.push(clean_type(ExtractedEvent::export_to_string()?));
    types.push(clean_type(ExtractedContact::export_to_string()?));
    types.push(clean_type(ExtractedLocation::export_to_string()?));
    types.push(clean_type(TextSpan::export_to_string()?));
    types.push(clean_type(TextSource::export_to_string()?));
    types.push(clean_type(Relationship::export_to_string()?));
    types.push(clean_type(RelationType::export_to_string()?));
    types.push(clean_type(EntityRef::export_to_string()?));
    types.push(clean_type(Ambiguity::export_to_string()?));
    types.push(clean_type(AmbiguityOption::export_to_string()?));

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
