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
 * Configuration for an AI provider API key
 */
export type AiProviderApiKeyConfig = { name: string, key: string | null, is_configured: boolean, };


/**
 * Configuration for an OAuth client app
 */
export type OAuthClientAppConfig = { provider: string, client_id: string | null, client_secret: string | null, is_configured: boolean, };


import type { AiProviderApiKeyConfig } from "./AiProviderApiKeyConfig";
import type { OAuthClientAppConfig } from "./OAuthClientAppConfig";

/**
 * Response for settings endpoint
 */
export type SettingsResponse = { config_file_path: string, ai_provider_api_keys: Array<AiProviderApiKeyConfig>, oauth_client_apps: Array<OAuthClientAppConfig>, projects_default_path: string | null, };


/**
 * Request to update AI provider API keys
 */
export type UpdateAiProviderApiKeysRequest = { gemini_api_key: string | null, };


/**
 * Request to update OAuth client apps
 */
export type UpdateOAuthClientAppsRequest = { google_client_id: string | null, google_client_secret: string | null, };


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
import type { JobType } from "./JobType";
import type { SourceType } from "./SourceType";

/**
 * Represents a long-running download job
 */
export type DownloadJob = { id: bigint, source_type: SourceType, credential_id: bigint, job_type: JobType, status: DownloadJobStatus, progress: DownloadProgress, error_message: string | null, created_at: bigint, started_at: bigint | null, updated_at: bigint, completed_at: bigint | null, last_sync_at: bigint | null, };


export type DownloadJobStatus = "pending" | "running" | "paused" | "completed" | "failed" | "cancelled";


export type DownloadProgress = { total_items: bigint, downloaded_items: bigint, failed_items: bigint, skipped_items: bigint, in_progress_items: bigint, remaining_items: bigint, percent_complete: number, bytes_downloaded: bigint, items_per_second: number, estimated_completion_secs: bigint | null, };


export type SourceType = "imap" | "google-drive" | "dropbox" | "one-drive" | "local-file";


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
export type Email = { id: bigint, download_item_id: bigint | null, credential_id: bigint, uid: number, folder_id: bigint, message_id: string | null, subject: string | null, from_address: string, from_name: string | null, to_addresses: Array<EmailAddress>, cc_addresses: Array<EmailAddress>, bcc_addresses: Array<EmailAddress>, reply_to: string | null, date_sent: bigint | null, date_received: bigint, body_text: string | null, body_html: string | null, is_read: boolean, is_flagged: boolean, is_draft: boolean, is_answered: boolean, has_attachments: boolean, attachment_count: number, size_bytes: number | null, thread_id: string | null, created_at: bigint, updated_at: bigint, };


export type EmailAddress = { email: string, name: string | null, };


import type { AttachmentExtractionStatus } from "./AttachmentExtractionStatus";

export type EmailAttachment = { id: bigint, email_id: bigint, filename: string, content_type: string | null, size_bytes: number | null, content_id: string | null, file_path: string, checksum: string | null, is_inline: boolean, extraction_status: AttachmentExtractionStatus, extracted_text: string | null, created_at: bigint, updated_at: bigint, };


export type AttachmentExtractionStatus = "pending" | "completed" | "failed" | "skipped";


/**
 * Request to list emails
 */
export type ListEmailsRequest = { credential_id: bigint | null, folder_id: bigint | null, label_id: bigint | null, limit: number | null, offset: number | null, search_query: string | null, };


import type { Email } from "./Email";

/**
 * Response for email list
 */
export type ListEmailsResponse = { emails: Array<Email>, total_count: bigint, has_more: boolean, };


/**
 * Request to fetch emails by IDs
 */
export type EmailsByIdsRequest = { email_ids: Array<bigint>, };


import type { Email } from "./Email";

/**
 * Response for email batch lookup
 */
export type EmailsByIdsResponse = { emails: Array<Email>, };


export type DocumentSourceType = "imap-account" | "local-folder" | "cloud-drive" | "cloud-mailbox" | "manual-import";


export type SourceAccessState = "accessible" | "offline" | "unreachable" | "disabled" | "unknown";


export type SourcePermissionState = "granted" | "expired" | "revoked" | "insufficient-scope" | "forbidden" | "unknown";


export type DocumentKind = "email" | "attachment" | "file";


import type { DocumentSourceType } from "./DocumentSourceType";
import type { SourceAccessState } from "./SourceAccessState";
import type { SourcePermissionState } from "./SourcePermissionState";

export type DocumentSource = { id: bigint, source_type: DocumentSourceType, display_name: string, credential_id: bigint | null, root_reference: string | null, access_state: SourceAccessState, permission_state: SourcePermissionState, access_checked_at: bigint | null, permission_checked_at: bigint | null, created_at: bigint, updated_at: bigint, };


import type { DocumentKind } from "./DocumentKind";

export type Document = { id: bigint, source_id: bigint, kind: DocumentKind, parent_document_id: bigint | null, email_id: bigint | null, attachment_id: bigint | null, title: string | null, canonical_name: string | null, mime_type: string | null, size_bytes: bigint | null, checksum_sha256: string | null, storage_path: string | null, external_uri: string | null, date_created: bigint | null, date_modified: bigint | null, date_received: bigint | null, indexed_at: bigint | null, created_at: bigint, updated_at: bigint, };


export type EmailFolder = { id: bigint, credential_id: bigint, name: string, display_name: string | null, imap_path: string, folder_type: string | null, parent_folder_id: bigint | null, uidvalidity: number | null, last_synced_uid: number | null, oldest_synced_uid: number | null, total_messages: number, unread_messages: number, is_subscribed: boolean, is_selectable: boolean, created_at: bigint, updated_at: bigint, last_synced_at: bigint | null, };


export type ListFoldersRequest = { credential_id: bigint, };


import type { EmailFolder } from "./EmailFolder";

export type ListFoldersResponse = { folders: Array<EmailFolder>, };


export type EmailLabel = { id: bigint, credential_id: bigint, name: string, display_name: string | null, label_type: string, color: string | null, message_count: number, created_at: bigint, updated_at: bigint, };


export type ListLabelsRequest = { credential_id: bigint, };


import type { EmailLabel } from "./EmailLabel";

export type ListLabelsResponse = { labels: Array<EmailLabel>, };


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


export type DataSourceType = "email" | "imap" | "bank-statement" | "credit-card-statement" | "bank-feed" | "csv-upload" | "manual" | "unknown";


export type FinancialDocumentType = "invoice" | "bill" | "bank-statement" | "receipt" | "tax-document" | "payment-confirmation";


import type { BillStatus } from "./BillStatus";
import type { DataSourceType } from "./DataSourceType";
import type { FinancialDocumentType } from "./FinancialDocumentType";

/**
 * A financial document (bill, invoice, receipt, statement) extracted from an email or file.
 * One Bill may be the source for zero (unpaid) or more FinancialTransactions.
 *
 * ## Date Column Conventions
 *
 * Every date has two columns:
 * - `{field}_raw`  — `TEXT` — the exact date string as it appeared in the source document
 *                    (e.g., "15 Jan 2025", "January 15th, 2025", "15/01/25")
 * - `{field}`      — `BIGINT` — parsed UTC timestamp in milliseconds since Unix epoch.
 *                    For date-only values, use 00:00:00 UTC for that calendar day.
 *                    Nullable when parsing fails.
 */
export type Bill = { id: bigint, data_source_type: DataSourceType, data_source_id: string, document_type: FinancialDocumentType, status: BillStatus, issuer_vendor_id: bigint | null, document_reference: string | null, total_amount: number | null, currency: string | null, 
/**
 * Date the bill or invoice was generated or issued by the vendor.
 * Distinct from due_date (when payment is expected) and billing_period (service window).
 * SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
 */
issued_date_raw: string | null, issued_date: bigint | null, 
/**
 * Date by which payment must be made.
 * SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
 */
due_date_raw: string | null, due_date: bigint | null, 
/**
 * Start and end of the billing or service period this bill covers.
 * SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
 */
billing_period_start_raw: string | null, billing_period_start: bigint | null, billing_period_end_raw: string | null, billing_period_end: bigint | null, created_at: bigint, updated_at: bigint, };


export type BillStatus = "received" | "unpaid" | "paid" | "overdue" | "cancelled";


import type { ServiceIdentifierKind } from "./ServiceIdentifierKind";

export type BillSubject = { id: bigint, bill_id: bigint, kind: ServiceIdentifierKind, value: string, masked_value: string | null, is_primary: boolean, created_at: bigint, updated_at: bigint, };


export type ServiceIdentifierKind = "phone-number" | "account-number" | "policy-number" | "meter-number" | "subscription-id" | "contract-id" | "other";


import type { DataSourceType } from "./DataSourceType";
import type { TransactionCategory } from "./TransactionCategory";
import type { TransactionParty } from "./TransactionParty";
import type { TransactionStatus } from "./TransactionStatus";

/**
 * Financial transaction extracted from documents
 */
export type FinancialTransaction = { id: bigint, data_source_type: DataSourceType, data_source_id: string, amount: number, currency: string, transaction_date: string, category: TransactionCategory | null, payer: TransactionParty, payee: TransactionParty, status: TransactionStatus, source_file: string | null, extracted_at: bigint, notes: string | null, transaction_reference: string | null, };


/**
 * Strongly typed transaction endpoint. Always present for both payer and payee.
 */
export type TransactionParty = { 
/**
 * Canonical vendor reference. Null means unresolved.
 */
vendor_id: bigint | null, };


export type TransactionCategory = "income" | "expense" | "investment" | "tax" | "utility" | "subscription" | "entertainment" | "travel" | "healthcare" | "education" | "other";


export type TransactionStatus = "paid" | "cancelled" | "refunded";


import type { VendorType } from "./VendorType";

/**
 * Transaction vendor entity
 */
export type Vendor = { id: bigint, vendor_type: VendorType, vendor_name: string, vendor_external_id: string | null, created_at: bigint, updated_at: bigint, };


export type VendorType = "self-user" | "self-business" | "financial-instrument" | "merchant" | "employer" | "bank" | "individual" | "platform" | "unknown";


/**
 * Financial summary/overview
 */
export type FinancialSummary = { total_income: number, total_expenses: number, net_balance: number, pending_bills: number, overdue_payments: number, currency: string, period_start: string, period_end: string, };


/**
 * Financial extraction source summary
 */
export type FinancialExtractionSummary = { source_count: bigint, transaction_count: bigint, last_extracted_at: bigint | null, };


import type { Bill } from "./Bill";
import type { CategoryBreakdown } from "./CategoryBreakdown";
import type { FinancialSummary } from "./FinancialSummary";
import type { FinancialTransaction } from "./FinancialTransaction";

/**
 * Financial health metrics
 */
export type FinancialHealth = { summary: FinancialSummary, recent_transactions: Array<FinancialTransaction>, upcoming_bills: Array<Bill>, category_breakdown: Array<CategoryBreakdown>, };


import type { TransactionCategory } from "./TransactionCategory";

/**
 * Breakdown by category
 */
export type CategoryBreakdown = { category: TransactionCategory, amount: number, percentage: number, transaction_count: number, };


export type FinancialPagination = { page: number, limit: number, total_count: number, total_pages: number, };


import type { Bill } from "./Bill";
import type { FinancialPagination } from "./FinancialPagination";

export type ListFinancialBillsResponse = { bills: Array<Bill>, pagination: FinancialPagination, };


export type FinancialTemplateType = "bill" | "transaction";


export type FinancialTemplateStatus = "active" | "superseded" | "disabled";


import type { DataSourceType } from "./DataSourceType";
import type { FinancialTemplateStatus } from "./FinancialTemplateStatus";
import type { FinancialTemplateType } from "./FinancialTemplateType";

export type FinancialExtractionTemplate = { id: bigint, data_source_type: DataSourceType, data_source_id: string, template_type: FinancialTemplateType, template_body: string, status: FinancialTemplateStatus, version: number, created_at: bigint, updated_at: bigint, };


export type FinancialTemplateVariable = { id: bigint, template_id: bigint, placeholder_name: string, target_field: string, created_at: bigint, };


import type { DataSourceType } from "./DataSourceType";

export type FinancialTemplateApplicability = { id: bigint, template_id: bigint, data_source_type: DataSourceType, data_source_id: string, match_score: number | null, created_at: bigint, };


export type DetectFinancialTemplatesRequest = { credential_id: bigint | null, max_candidate_emails: number | null, max_templates_per_sender: number | null, };


export type DetectedFinancialTemplateVariable = { placeholder_name: string, target_field: string, };


import type { DetectedFinancialTemplateVariable } from "./DetectedFinancialTemplateVariable";
import type { FinancialTemplateType } from "./FinancialTemplateType";

export type DetectedFinancialTemplate = { template_id: bigint, sender_email: string, template_type: FinancialTemplateType, template_body: string, translated_template_body: string, source_email_ids: Array<bigint>, variables: Array<DetectedFinancialTemplateVariable>, };


import type { DetectedFinancialTemplate } from "./DetectedFinancialTemplate";

export type DetectFinancialTemplatesResponse = { candidate_sender_count: number, candidate_email_count: number, templates: Array<DetectedFinancialTemplate>, };


export type TemplateDetectionSenderRank = { sender_email: string, rank: number, total_candidate_emails: number, recent_candidate_emails: number, latest_email_ts: bigint, max_existing_cluster_size: number, };


import type { DetectedFinancialTemplateVariable } from "./DetectedFinancialTemplateVariable";
import type { FinancialTemplateType } from "./FinancialTemplateType";

export type TemplateDetectionGeneratedTemplateDebug = { template_id: bigint | null, template_type: FinancialTemplateType | null, template_body: string, translated_template_body: string, source_email_ids: Array<bigint>, variables: Array<DetectedFinancialTemplateVariable>, has_bill: boolean, discarded_reason: string | null, };


import type { TemplateDetectionGeneratedTemplateDebug } from "./TemplateDetectionGeneratedTemplateDebug";

export type TemplateDetectionSenderDebug = { sender_email: string, rank: number, sender_candidate_count: number, existing_template_count: number, initially_matched_count: number, fresh_unmatched_count: number, pool_count: number, generated_templates: Array<TemplateDetectionGeneratedTemplateDebug>, error: string | null, skipped_reason: string | null, };


import type { TemplateDetectionSenderDebug } from "./TemplateDetectionSenderDebug";
import type { TemplateDetectionSenderRank } from "./TemplateDetectionSenderRank";

export type TemplateDetectionDebugState = { keyword_query: string, keyword_list: Array<string>, max_candidate_emails: number, matched_document_ids_count: number, sender_ranking: Array<TemplateDetectionSenderRank>, candidate_email_ids: Array<bigint>, sender_debug: Array<TemplateDetectionSenderDebug>, };


export type FinancialTemplateDetectionJobStatus = "idle" | "running" | "completed" | "failed";


import type { FinancialTemplateDetectionJobStatus } from "./FinancialTemplateDetectionJobStatus";
import type { TemplateDetectionDebugState } from "./TemplateDetectionDebugState";

export type FinancialTemplateDetectionJobState = { run_id: bigint, status: FinancialTemplateDetectionJobStatus, started_at: bigint | null, finished_at: bigint | null, total_senders: number, processed_senders: number, current_sender: string | null, candidate_sender_count: number, candidate_email_count: number, new_templates_count: number, error: string | null, debug: TemplateDetectionDebugState | null, };


export type TemplateDetectionSenderLlmDraftPreview = { seed_text: string, cluster_size: number, selected_email_ids: Array<bigint>, full_template: string, sample_subject: string, sample_body: string, };


import type { TemplateDetectionSenderLlmDraftPreview } from "./TemplateDetectionSenderLlmDraftPreview";

export type TemplateDetectionSenderLlmInputsResponse = { sender_email: string, sender_candidate_count: number, existing_template_count: number, initially_matched_count: number, fresh_unmatched_count: number, pool_count: number, drafts: Array<TemplateDetectionSenderLlmDraftPreview>, };


export type FinancialTemplateFieldMapping = { placeholder_name: string, target_field: string, };


import type { FinancialExtractionTemplate } from "./FinancialExtractionTemplate";
import type { FinancialTemplateFieldMapping } from "./FinancialTemplateFieldMapping";

export type FinancialTemplateWithVariables = { template: FinancialExtractionTemplate, variables: Array<FinancialTemplateFieldMapping>, };


import type { FinancialTemplateWithVariables } from "./FinancialTemplateWithVariables";

export type ListFinancialTemplatesResponse = { templates: Array<FinancialTemplateWithVariables>, };


export type DeleteFinancialTemplatesRequest = { template_ids: Array<bigint>, };


export type DeleteFinancialTemplatesResponse = { deleted_count: number, };
