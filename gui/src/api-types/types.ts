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
