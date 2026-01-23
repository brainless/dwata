use serde::{Deserialize, Serialize};

pub mod event;
pub mod extraction; // NEW
pub mod project;
pub mod session;
pub mod settings;
pub mod task;

pub use event::{CreateEventRequest, Event, EventsResponse, UpdateEventRequest};
pub use project::{
    CreateProjectRequest, Project, ProjectStatus, ProjectsResponse, UpdateProjectRequest,
};
pub use session::{
    AgentMessage, AgentSession, AgentToolCall, SessionListItem, SessionListResponse,
    SessionMessage, SessionResponse, SessionToolCall,
};
pub use settings::{ApiKeyConfig, SettingsResponse, UpdateApiKeysRequest};
pub use task::{
    CreateTaskRequest, Task, TaskPriority, TaskStatus, TasksResponse, UpdateTaskRequest,
};

// Re-export extraction types
pub use extraction::*;

/// Error response for API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Request to create a new agent session
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_name: String,
    pub user_prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub config: Option<serde_json::Value>,
}

/// Response after creating a session
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: i64,
    pub agent_name: String,
    pub status: String,
}
