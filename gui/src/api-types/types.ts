/**
 * Project entity for managing work and hobby projects
 */
export type Project = { id: number, name: string, description: string, status: ProjectStatus, tasks_completed: number, tasks_total: number, deadline: string | null, notifications: number, created_at: bigint, updated_at: bigint, };


export type ProjectStatus = "active" | "planning" | "on-hold" | "completed" | "archived";


/**
 * Request to create a new project
 */
export type CreateProjectRequest = { name: string, description: string, deadline: string | null, };


/**
 * Request to update a project
 */
export type UpdateProjectRequest = { name: string | null, description: string | null, status: ProjectStatus | null, deadline: string | null, };


/**
 * Response containing a list of projects
 */
export type ProjectsResponse = { projects: Array<Project>, };


/**
 * Event entity for calendar events
 */
export type Event = { id: number, name: string, description: string | null, project_id: number | null, task_id: number | null, date: string, attendee_user_ids: Array<number> | null, created_at: bigint, updated_at: bigint, };


/**
 * Request to create a new event
 */
export type CreateEventRequest = { name: string, description: string | null, project_id: number | null, task_id: number | null, date: string, attendee_user_ids: Array<number> | null, };


/**
 * Request to update an event
 */
export type UpdateEventRequest = { name: string | null, description: string | null, project_id: number | null, task_id: number | null, date: string | null, attendee_user_ids: Array<number> | null, };


/**
 * Response containing a list of events
 */
export type EventsResponse = { events: Array<Event>, };


/**
 * Task entity for managing individual tasks
 */
export type Task = { id: number, project_id: number | null, title: string, description: string | null, status: TaskStatus, priority: TaskPriority, due_date: string | null, assigned_to: string | null, created_at: bigint, updated_at: bigint, };


export type TaskStatus = "todo" | "in-progress" | "review" | "done" | "cancelled";


export type TaskPriority = "low" | "medium" | "high" | "critical";


/**
 * Request to create a new task
 */
export type CreateTaskRequest = { project_id: number | null, title: string, description: string | null, priority: TaskPriority, due_date: string | null, };


/**
 * Request to update a task
 */
export type UpdateTaskRequest = { project_id: number | null, title: string | null, description: string | null, status: TaskStatus | null, priority: TaskPriority | null, due_date: string | null, };


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


/**
 * Detailed session with messages and tool calls
 */
export type SessionResponse = { id: bigint, agent_name: string, provider: string, model: string, system_prompt: string | null, user_prompt: string, config: any, status: string, result: string | null, messages: Array<SessionMessage>, tool_calls: Array<SessionToolCall>, started_at: bigint, ended_at: bigint | null, };


/**
 * Simplified session info for list views
 */
export type SessionListItem = { id: bigint, agent_name: string, user_prompt: string, status: string, started_at: bigint, };


/**
 * List of sessions response
 */
export type SessionListResponse = { sessions: Array<SessionListItem>, };


/**
 * Configuration for an API key
 */
export type ApiKeyConfig = { name: string, key: string | null, is_configured: boolean, };


/**
 * Response for settings endpoint
 */
export type SettingsResponse = { config_file_path: string, api_keys: Array<ApiKeyConfig>, projects_default_path: string | null, };


/**
 * Request to update API keys
 */
export type UpdateApiKeysRequest = { gemini_api_key: string | null, };


export type CredentialType = "imap" | "smtp" | "oauth" | "apikey" | "database" | "custom";


export type CreateCredentialRequest = { credential_type: CredentialType, identifier: string, username: string, password: string, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, extra_metadata: string | null, };


export type UpdateCredentialRequest = { username: string | null, password: string | null, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, extra_metadata: string | null, };


export type CredentialMetadata = { id: bigint, credential_type: CredentialType, identifier: string, username: string, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, created_at: bigint, updated_at: bigint, last_accessed_at: bigint | null, is_active: boolean, extra_metadata: string | null, };


export type PasswordResponse = { password: string, };


export type CredentialListResponse = { credentials: Array<CredentialMetadata>, };


export type ImapAuthMethod = "plain" | "oauth2" | "xoauth2";


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


/**
 * Type-safe request for creating IMAP credentials
 */
export type CreateImapCredentialRequest = { identifier: string, username: string, password: string, settings: ImapAccountSettings, notes: string | null, };


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
 * Represents a long-running download job
 */
export type DownloadJob = { id: bigint, source_type: SourceType, credential_id: bigint, status: DownloadJobStatus, progress: DownloadProgress, error_message: string | null, created_at: bigint, started_at: bigint | null, updated_at: bigint, completed_at: bigint | null, last_sync_at: bigint | null, };


export type DownloadJobStatus = "pending" | "running" | "paused" | "completed" | "failed" | "cancelled";


export type DownloadProgress = { total_items: bigint, downloaded_items: bigint, failed_items: bigint, skipped_items: bigint, in_progress_items: bigint, remaining_items: bigint, percent_complete: number, bytes_downloaded: bigint, items_per_second: number, estimated_completion_secs: bigint | null, };


export type SourceType = "imap" | "google-drive" | "dropbox" | "one-drive";


/**
 * IMAP-specific download state
 */
export type ImapDownloadState = { folders: Array<ImapFolderStatus>, sync_strategy: ImapSyncStrategy, fetch_batch_size: number, };


export type ImapFolderStatus = { name: string, total_messages: number, downloaded_messages: number, failed_messages: number, skipped_messages: number, last_synced_uid: number | null, is_complete: boolean, };


export type ImapSyncStrategy = "full-sync" | "inbox-only" | { "selected-folders": Array<string> } | "new-only" | { "date-range": { from: string, to: string, } };


/**
 * Cloud storage-specific state (for future)
 */
export type CloudStorageDownloadState = { root_path: string, directories: Array<DirectoryStatus>, file_filter: FileFilter | null, };


export type DirectoryStatus = { path: string, total_files: number, downloaded_files: number, failed_files: number, is_complete: boolean, };


export type FileFilter = { extensions: Array<string> | null, pattern: string | null, min_size_bytes: bigint | null, max_size_bytes: bigint | null, };


/**
 * Request to create a new download job
 */
export type CreateDownloadJobRequest = { credential_id: bigint, source_type: SourceType, };


/**
 * Response for download job list
 */
export type DownloadJobListResponse = { jobs: Array<DownloadJob>, };


/**
 * Individual download item
 */
export type DownloadItem = { id: bigint, job_id: bigint, source_identifier: string, source_folder: string | null, item_type: string, status: DownloadItemStatus, size_bytes: bigint | null, error_message: string | null, created_at: bigint, downloaded_at: bigint | null, };


export type DownloadItemStatus = "pending" | "downloading" | "completed" | "failed" | "skipped";


export type DataType = "project" | "task" | "event" | "contact" | "location" | "date" | "priority" | "status";


export type ExtractionMethod = "attachment-parsing" | "pattern-based" | "gliner-ner" | "bert-ner" | "llm-based" | "hybrid";


export type Attachment = { filename: string, content_type: string, content: Array<number>, };


export type EmailAddress = { email: string, name: string | null, };


export type UserPreferences = { date_format: string, default_task_priority: TaskPriority, default_project_status: ProjectStatus, auto_link_threshold: number, };


export type ExtractedEntity = { "type": "Project", "data": ExtractedProject } | { "type": "Task", "data": ExtractedTask } | { "type": "Event", "data": ExtractedEvent } | { "type": "Contact", "data": ExtractedContact } | { "type": "Location", "data": ExtractedLocation };


export type ExtractedProject = { name: string, description: string | null, deadline: string | null, status: ProjectStatus | null, };


export type ExtractedTask = { title: string, description: string | null, priority: TaskPriority | null, due_date: string | null, assigned_to: string | null, project_id: number | null, };


export type ExtractedEvent = { name: string, description: string | null, date: string, location: string | null, attendees: Array<string>, project_id: number | null, task_id: number | null, };


export type ExtractedContact = { name: string, email: string | null, phone: string | null, organization: string | null, };


export type ExtractedLocation = { name: string, address: string | null, coordinates: [number, number] | null, };


/**
 * Location of text in email
 */
export type TextSpan = { source: TextSource, start: number, end: number, text: string, };


export type TextSource = { "type": "Subject" } | { "type": "Body" } | { "type": "Attachment", "data": string };


/**
 * Relationship between entities
 */
export type Relationship = { relation_type: RelationType, target_entity: EntityRef, confidence: number, };


export type RelationType = "belongs-to-project" | "linked-to-task" | "assigned-to" | "located-at" | "has-deadline";


export type EntityRef = { data_type: DataType, entity_id: number | null, extracted_index: number | null, };


/**
 * Ambiguity in extraction
 */
export type Ambiguity = { field: string, options: Array<AmbiguityOption>, reason: string, };


export type AmbiguityOption = { value: string, confidence: number, };
