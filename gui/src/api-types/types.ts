/**
 * Project entity for managing work and hobby projects
 */
export type Project = {
  id: number;
  name: string;
  description: string;
  status: ProjectStatus;
  tasks_completed: number;
  tasks_total: number;
  deadline: string | null;
  notifications: number;
  created_at: bigint;
  updated_at: bigint;
};

export type ProjectStatus =
  | "active"
  | "planning"
  | "on-hold"
  | "completed"
  | "archived";

/**
 * Request to create a new project
 */
export type CreateProjectRequest = {
  name: string;
  description: string;
  deadline: string | null;
};

/**
 * Request to update a project
 */
export type UpdateProjectRequest = {
  name: string | null;
  description: string | null;
  status: ProjectStatus | null;
  deadline: string | null;
};

/**
 * Response containing a list of projects
 */
export type ProjectsResponse = { projects: Array<Project> };

/**
 * Task entity for managing individual tasks
 */
export type Task = {
  id: number;
  project_id: number | null;
  title: string;
  description: string | null;
  status: TaskStatus;
  priority: TaskPriority;
  due_date: string | null;
  assigned_to: string | null;
  created_at: bigint;
  updated_at: bigint;
};

export type TaskStatus =
  | "todo"
  | "in-progress"
  | "review"
  | "done"
  | "cancelled";

export type TaskPriority = "low" | "medium" | "high" | "critical";

/**
 * Request to create a new task
 */
export type CreateTaskRequest = {
  project_id: number | null;
  title: string;
  description: string | null;
  priority: TaskPriority;
  due_date: string | null;
};

/**
 * Request to update a task
 */
export type UpdateTaskRequest = {
  project_id: number | null;
  title: string | null;
  description: string | null;
  status: TaskStatus | null;
  priority: TaskPriority | null;
  due_date: string | null;
};

/**
 * Response containing a list of tasks
 */
export type TasksResponse = { tasks: Array<Task> };

/**
 * Message in session response
 */
export type SessionMessage = {
  role: string;
  content: string;
  created_at: bigint;
};

/**
 * Tool call in session response
 */
export type SessionToolCall = {
  tool_name: string;
  request: any;
  response: any;
  status: string;
  execution_time_ms: bigint | null;
};

/**
 * Detailed session with messages and tool calls
 */
export type SessionResponse = {
  id: bigint;
  agent_name: string;
  provider: string;
  model: string;
  system_prompt: string | null;
  user_prompt: string;
  config: any;
  status: string;
  result: string | null;
  messages: Array<SessionMessage>;
  tool_calls: Array<SessionToolCall>;
  started_at: bigint;
  ended_at: bigint | null;
};

/**
 * Simplified session info for list views
 */
export type SessionListItem = {
  id: bigint;
  agent_name: string;
  user_prompt: string;
  status: string;
  started_at: bigint;
};

/**
 * List of sessions response
 */
export type SessionListResponse = { sessions: Array<SessionListItem> };

/**
 * Configuration for an API key
 */
export type ApiKeyConfig = {
  name: string;
  key: string | null;
  is_configured: boolean;
};

/**
 * Response for settings endpoint
 */
export type SettingsResponse = {
  config_file_path: string;
  api_keys: Array<ApiKeyConfig>;
  projects_default_path: string | null;
};

/**
 * Request to update API keys
 */
export type UpdateApiKeysRequest = { gemini_api_key: string | null };
