import type { ProjectStatus } from "./ProjectStatus";

/**
 * Project entity for managing work and hobby projects
 */
export type Project = { id: number, name: string, description: string, status: ProjectStatus, tasks_completed: number, tasks_total: number, deadline: string | null, notifications: number, created_at: bigint, updated_at: bigint, };


export type ProjectStatus = "active" | "planning" | "on-hold" | "completed" | "archived";


/**
 * Request to create a new project
 */
export type CreateProjectRequest = { name: string, description: string, deadline: string | null, };


import type { ProjectStatus } from "./ProjectStatus";

/**
 * Request to update a project
 */
export type UpdateProjectRequest = { name: string | null, description: string | null, status: ProjectStatus | null, deadline: string | null, };


import type { Project } from "./Project";

/**
 * Response containing a list of projects
 */
export type ProjectsResponse = { projects: Array<Project>, };


export type Event = { id: bigint, extraction_job_id: bigint | null, email_id: bigint | null, name: string, description: string | null, event_date: bigint, location: string | null, confidence: number | null, requires_review: boolean, is_confirmed: boolean, project_id: bigint | null, task_id: bigint | null, created_at: bigint, updated_at: bigint, };


export type CreateEventRequest = { name: string, description: string | null, event_date: bigint, location: string | null, attendees: Array<string>, };


export type UpdateEventRequest = { name: string | null, description: string | null, event_date: bigint | null, location: string | null, attendees: Array<string> | null, is_confirmed: boolean | null, };


import type { Event } from "./Event";

export type EventsResponse = { events: Array<Event>, };


import type { TaskPriority } from "./TaskPriority";
import type { TaskStatus } from "./TaskStatus";

/**
 * Task entity for managing individual tasks
 */
export type Task = { id: number, project_id: number | null, title: string, description: string | null, status: TaskStatus, priority: TaskPriority, due_date: string | null, assigned_to: string | null, created_at: bigint, updated_at: bigint, };


export type TaskStatus = "todo" | "in-progress" | "review" | "done" | "cancelled";


export type TaskPriority = "low" | "medium" | "high" | "critical";


import type { TaskPriority } from "./TaskPriority";

/**
 * Request to create a new task
 */
export type CreateTaskRequest = { project_id: number | null, title: string, description: string | null, priority: TaskPriority, due_date: string | null, };


import type { TaskPriority } from "./TaskPriority";
import type { TaskStatus } from "./TaskStatus";

/**
 * Request to update a task
 */
export type UpdateTaskRequest = { project_id: number | null, title: string | null, description: string | null, status: TaskStatus | null, priority: TaskPriority | null, due_date: string | null, };


import type { Task } from "./Task";

/**
 * Response containing a list of tasks
 */
export type TasksResponse = { tasks: Array<Task>, };


/**
 * Message in session response
 */
export type SessionMessage = { role: string, content: string, created_at: bigint, };


/**
 * Tool call in session response
 */
export type SessionToolCall = { tool_name: string, request: any, response: any, status: string, execution_time_ms: bigint | null, };


import type { SessionMessage } from "./SessionMessage";
import type { SessionToolCall } from "./SessionToolCall";

/**
 * Detailed session with messages and tool calls
 */
export type SessionResponse = { id: bigint, agent_name: string, provider: string, model: string, system_prompt: string | null, user_prompt: string, config: any, status: string, result: string | null, messages: Array<SessionMessage>, tool_calls: Array<SessionToolCall>, started_at: bigint, ended_at: bigint | null, };


/**
 * Simplified session info for list views
 */
export type SessionListItem = { id: bigint, agent_name: string, user_prompt: string, status: string, started_at: bigint, };


import type { SessionListItem } from "./SessionListItem";

/**
 * List of sessions response
 */
export type SessionListResponse = { sessions: Array<SessionListItem>, };


/**
 * Configuration for an API key
 */
export type ApiKeyConfig = { name: string, key: string | null, is_configured: boolean, };


import type { ApiKeyConfig } from "./ApiKeyConfig";

/**
 * Response for settings endpoint
 */
export type SettingsResponse = { config_file_path: string, api_keys: Array<ApiKeyConfig>, projects_default_path: string | null, };


/**
 * Request to update API keys
 */
export type UpdateApiKeysRequest = { gemini_api_key: string | null, };


export type CredentialType = "imap" | "smtp" | "oauth" | "apikey" | "database" | "localfile" | "custom";


import type { CredentialType } from "./CredentialType";

export type CreateCredentialRequest = { credential_type: CredentialType, identifier: string, username: string, 
/**
 * Password is optional for credential types that don't require keychain storage (e.g., LocalFile)
 */
password: string | null, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, extra_metadata: string | null, };


export type UpdateCredentialRequest = { username: string | null, password: string | null, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, extra_metadata: string | null, };


import type { CredentialType } from "./CredentialType";

export type CredentialMetadata = { id: bigint, credential_type: CredentialType, identifier: string, username: string, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, created_at: bigint, updated_at: bigint, last_accessed_at: bigint | null, is_active: boolean, extra_metadata: string | null, };


export type PasswordResponse = { password: string, };


import type { CredentialMetadata } from "./CredentialMetadata";

export type CredentialListResponse = { credentials: Array<CredentialMetadata>, };


export type ImapAuthMethod = "plain" | "oauth2" | "xoauth2";


import type { ImapAuthMethod } from "./ImapAuthMethod";

/**
 * IMAP-specific account settings
 */
export type ImapAccountSettings = { 
/**
 * IMAP server host (e.g., "imap.gmail.com")
 */
host: string, 
/**
 * IMAP server port (typically 993 for SSL, 143 for non-SSL)
 */
port: number, 
/**
 * Use TLS/SSL connection
 */
use_tls: boolean, 
/**
 * Authentication method
 */
auth_method: ImapAuthMethod, 
/**
 * Default mailbox/folder to monitor (default: "INBOX")
 */
default_mailbox: string, 
/**
 * Connection timeout in seconds
 */
connection_timeout_secs: number, 
/**
 * Whether to validate SSL certificates (should be true in production)
 */
validate_certs: boolean, };


import type { ImapAccountSettings } from "./ImapAccountSettings";

/**
 * Type-safe request for creating IMAP credentials
 */
export type CreateImapCredentialRequest = { identifier: string, username: string, password: string, settings: ImapAccountSettings, notes: string | null, };


import type { ImapAccountSettings } from "./ImapAccountSettings";

/**
 * Extended credential metadata with parsed IMAP settings
 */
export type ImapCredentialMetadata = { id: bigint, identifier: string, username: string, settings: ImapAccountSettings, notes: string | null, created_at: bigint, updated_at: bigint, last_accessed_at: bigint | null, is_active: boolean, };


/**
 * SMTP-specific account settings
 */
export type SmtpAccountSettings = { 
/**
 * SMTP server host (e.g., "smtp.gmail.com")
 */
host: string, 
/**
 * SMTP server port (typically 587 for STARTTLS, 465 for SSL)
 */
port: number, 
/**
 * Use TLS/SSL connection
 */
use_tls: boolean, 
/**
 * Connection timeout in seconds
 */
connection_timeout_secs: number, };


/**
 * API Key service settings
 */
export type ApiKeySettings = { 
/**
 * Base URL for the API (e.g., "https://api.stripe.com")
 */
base_url: string, 
/**
 * API version (if applicable)
 */
api_version: string | null, 
/**
 * Request timeout in seconds
 */
timeout_secs: number, };


/**
 * Local file path settings
 */
export type LocalFileSettings = { 
/**
 * Absolute path to the file or directory
 */
file_path: string, 
/**
 * Optional description of what this file contains
 */
description: string | null, 
/**
 * File type hint (e.g., "linkedin-archive", "email-export")
 */
file_type: string | null, };


import type { LocalFileSettings } from "./LocalFileSettings";

/**
 * Type-safe request for creating local file credentials
 */
export type CreateLocalFileCredentialRequest = { identifier: string, settings: LocalFileSettings, notes: string | null, };


import type { DownloadJobStatus } from "./DownloadJobStatus";
import type { DownloadProgress } from "./DownloadProgress";
import type { SourceType } from "./SourceType";

/**
 * Represents a long-running download job
 */
export type DownloadJob = { id: bigint, source_type: SourceType, credential_id: bigint, status: DownloadJobStatus, progress: DownloadProgress, error_message: string | null, created_at: bigint, started_at: bigint | null, updated_at: bigint, completed_at: bigint | null, last_sync_at: bigint | null, };


export type DownloadJobStatus = "pending" | "running" | "paused" | "completed" | "failed" | "cancelled";


export type DownloadProgress = { total_items: bigint, downloaded_items: bigint, failed_items: bigint, skipped_items: bigint, in_progress_items: bigint, remaining_items: bigint, percent_complete: number, bytes_downloaded: bigint, items_per_second: number, estimated_completion_secs: bigint | null, };


export type SourceType = "imap" | "google-drive" | "dropbox" | "one-drive";


import type { ImapFolderStatus } from "./ImapFolderStatus";
import type { ImapSyncStrategy } from "./ImapSyncStrategy";

/**
 * IMAP-specific download state
 */
export type ImapDownloadState = { folders: Array<ImapFolderStatus>, sync_strategy: ImapSyncStrategy, fetch_batch_size: number, max_age_months: number | null, };


export type ImapFolderStatus = { name: string, total_messages: number, downloaded_messages: number, failed_messages: number, skipped_messages: number, last_synced_uid: number | null, is_complete: boolean, };


export type ImapSyncStrategy = "full-sync" | "inbox-only" | { "selected-folders": Array<string> } | "new-only" | { "date-range": { from: string, to: string, } };


import type { DirectoryStatus } from "./DirectoryStatus";
import type { FileFilter } from "./FileFilter";

/**
 * Cloud storage-specific state (for future)
 */
export type CloudStorageDownloadState = { root_path: string, directories: Array<DirectoryStatus>, file_filter: FileFilter | null, };


export type DirectoryStatus = { path: string, total_files: number, downloaded_files: number, failed_files: number, is_complete: boolean, };


export type FileFilter = { extensions: Array<string> | null, pattern: string | null, min_size_bytes: bigint | null, max_size_bytes: bigint | null, };


import type { SourceType } from "./SourceType";

/**
 * Request to create a new download job
 */
export type CreateDownloadJobRequest = { credential_id: bigint, source_type: SourceType, };


import type { DownloadJob } from "./DownloadJob";

/**
 * Response for download job list
 */
export type DownloadJobListResponse = { jobs: Array<DownloadJob>, };


import type { DownloadItemStatus } from "./DownloadItemStatus";

/**
 * Individual download item
 */
export type DownloadItem = { id: bigint, job_id: bigint, source_identifier: string, source_folder: string | null, item_type: string, status: DownloadItemStatus, size_bytes: bigint | null, error_message: string | null, created_at: bigint, downloaded_at: bigint | null, };


export type DownloadItemStatus = "pending" | "downloading" | "completed" | "failed" | "skipped";


import type { EmailAddress } from "./EmailAddress";

/**
 * Represents a stored email
 */
export type Email = { id: bigint, download_item_id: bigint | null, credential_id: bigint, uid: number, folder: string, message_id: string | null, subject: string | null, from_address: string, from_name: string | null, to_addresses: Array<EmailAddress>, cc_addresses: Array<EmailAddress>, bcc_addresses: Array<EmailAddress>, reply_to: string | null, date_sent: bigint | null, date_received: bigint, body_text: string | null, body_html: string | null, is_read: boolean, is_flagged: boolean, is_draft: boolean, is_answered: boolean, has_attachments: boolean, attachment_count: number, size_bytes: number | null, thread_id: string | null, labels: Array<string>, created_at: bigint, updated_at: bigint, };


export type EmailAddress = { email: string, name: string | null, };


import type { AttachmentExtractionStatus } from "./AttachmentExtractionStatus";

export type EmailAttachment = { id: bigint, email_id: bigint, filename: string, content_type: string | null, size_bytes: number | null, content_id: string | null, file_path: string, checksum: string | null, is_inline: boolean, extraction_status: AttachmentExtractionStatus, extracted_text: string | null, created_at: bigint, updated_at: bigint, };


export type AttachmentExtractionStatus = "pending" | "completed" | "failed" | "skipped";


/**
 * Request to list emails
 */
export type ListEmailsRequest = { credential_id: bigint | null, folder: string | null, limit: number | null, offset: number | null, search_query: string | null, };


import type { Email } from "./Email";

/**
 * Response for email list
 */
export type ListEmailsResponse = { emails: Array<Email>, total_count: bigint, has_more: boolean, };


export type DataType = "project" | "task" | "event" | "contact" | "location" | "date" | "priority" | "status" | "company" | "position";


export type ExtractionMethod = "attachment-parsing" | "pattern-based" | "gliner-ner" | "bert-ner" | "llm-based" | "hybrid";


export type Attachment = { filename: string, content_type: string, content: Array<number>, };


import type { ProjectStatus } from "./ProjectStatus";
import type { TaskPriority } from "./TaskPriority";

export type UserPreferences = { date_format: string, default_task_priority: TaskPriority, default_project_status: ProjectStatus, auto_link_threshold: number, };


import type { ExtractedCompany } from "./ExtractedCompany";
import type { ExtractedContact } from "./ExtractedContact";
import type { ExtractedEvent } from "./ExtractedEvent";
import type { ExtractedLocation } from "./ExtractedLocation";
import type { ExtractedPosition } from "./ExtractedPosition";
import type { ExtractedProject } from "./ExtractedProject";
import type { ExtractedTask } from "./ExtractedTask";

export type ExtractedEntity = { "type": "Project", "data": ExtractedProject } | { "type": "Task", "data": ExtractedTask } | { "type": "Event", "data": ExtractedEvent } | { "type": "Contact", "data": ExtractedContact } | { "type": "Location", "data": ExtractedLocation } | { "type": "Company", "data": ExtractedCompany } | { "type": "Position", "data": ExtractedPosition };


import type { ProjectStatus } from "./ProjectStatus";

export type ExtractedProject = { name: string, description: string | null, deadline: string | null, status: ProjectStatus | null, };


import type { TaskPriority } from "./TaskPriority";

export type ExtractedTask = { title: string, description: string | null, priority: TaskPriority | null, due_date: string | null, assigned_to: string | null, project_id: number | null, };


export type ExtractedEvent = { name: string, description: string | null, date: string, location: string | null, attendees: Array<string>, project_id: number | null, task_id: number | null, };


import type { ProfileUrl } from "./ProfileUrl";

export type ExtractedContact = { name: string, email: string | null, phone: string | null, organization: string | null, profile_urls: Array<ProfileUrl>, };


export type ExtractedLocation = { name: string, address: string | null, coordinates: [number, number] | null, };


import type { TextSource } from "./TextSource";

/**
 * Location of text in email
 */
export type TextSpan = { source: TextSource, start: number, end: number, text: string, };


export type TextSource = { "type": "Subject" } | { "type": "Body" } | { "type": "Attachment", "data": string };


import type { EntityRef } from "./EntityRef";
import type { RelationType } from "./RelationType";

/**
 * Relationship between entities
 */
export type Relationship = { relation_type: RelationType, target_entity: EntityRef, confidence: number, };


export type RelationType = "belongs-to-project" | "linked-to-task" | "assigned-to" | "located-at" | "has-deadline";


import type { DataType } from "./DataType";

export type EntityRef = { data_type: DataType, entity_id: number | null, extracted_index: number | null, };


import type { AmbiguityOption } from "./AmbiguityOption";

/**
 * Ambiguity in extraction
 */
export type Ambiguity = { field: string, options: Array<AmbiguityOption>, reason: string, };


export type AmbiguityOption = { value: string, confidence: number, };


import type { ExtractionJobStatus } from "./ExtractionJobStatus";
import type { ExtractionProgress } from "./ExtractionProgress";
import type { ExtractionSourceType } from "./ExtractionSourceType";
import type { ExtractorType } from "./ExtractorType";

/**
 * Extraction job for processing attachments and extracting entities
 */
export type ExtractionJob = { id: bigint, source_type: ExtractionSourceType, extractor_type: ExtractorType, status: ExtractionJobStatus, progress: ExtractionProgress, error_message: string | null, created_at: bigint, started_at: bigint | null, updated_at: bigint, completed_at: bigint | null, };


export type ExtractionJobStatus = "pending" | "running" | "completed" | "failed" | "cancelled";


export type ExtractionSourceType = "email-attachment" | "local-file" | "local-archive" | "email-body";


export type ExtractorType = "attachment-parser" | "linked-in-archive" | "gliner-ner" | "llm-based";


export type ExtractionProgress = { total_items: bigint, processed_items: bigint, extracted_entities: bigint, failed_items: bigint, events_extracted: bigint, contacts_extracted: bigint, companies_extracted: bigint, positions_extracted: bigint, percent_complete: number, };


import type { ArchiveType } from "./ArchiveType";
import type { AttachmentExtractionFilter } from "./AttachmentExtractionFilter";

export type ExtractionSourceConfig = { "type": "EmailAttachments", "config": { email_ids: Array<bigint> | null, attachment_types: Array<string>, status_filter: AttachmentExtractionFilter, } } | { "type": "LocalFile", "config": { file_path: string, content_type: string, } } | { "type": "LocalArchive", "config": { archive_path: string, archive_type: ArchiveType, files_to_process: Array<string>, } };


export type AttachmentExtractionFilter = "pending" | "pending-and-failed" | "all";


import type { ExtractionSourceConfig } from "./ExtractionSourceConfig";
import type { ExtractionSourceType } from "./ExtractionSourceType";
import type { ExtractorType } from "./ExtractorType";

/**
 * Request to create extraction job
 */
export type CreateExtractionJobRequest = { source_type: ExtractionSourceType, extractor_type: ExtractorType, source_config: ExtractionSourceConfig, };


import type { ExtractionJob } from "./ExtractionJob";

/**
 * Response for extraction job list
 */
export type ExtractionJobListResponse = { jobs: Array<ExtractionJob>, };


export type Contact = { id: bigint, extraction_job_id: bigint | null, email_id: bigint | null, name: string, email: string | null, phone: string | null, organization: string | null, confidence: number | null, requires_review: boolean, is_confirmed: boolean, is_duplicate: boolean, merged_into_contact_id: bigint | null, created_at: bigint, updated_at: bigint, };


export type CreateContactRequest = { name: string, email: string | null, phone: string | null, organization: string | null, };


export type UpdateContactRequest = { name: string | null, email: string | null, phone: string | null, organization: string | null, is_confirmed: boolean | null, };


import type { Contact } from "./Contact";

export type ContactsResponse = { contacts: Array<Contact>, };
