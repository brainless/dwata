use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Core agent session model stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: i64,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub config: Option<serde_json::Value>,
    pub status: String, // 'running', 'completed', 'failed'
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Message in an agent session (user, assistant, system, tool)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String, // 'user', 'assistant', 'system', 'tool'
    pub content: String,
    pub created_at: i64,
}

/// Tool/function call made during agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCall {
    pub id: i64,
    pub session_id: i64,
    pub message_id: Option<i64>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub status: String, // 'pending', 'executing', 'completed', 'failed'
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub execution_time_ms: Option<i64>,
    pub error_details: Option<String>,
}

impl AgentToolCall {
    pub fn complete(&mut self, response: serde_json::Value, execution_time_ms: i64) {
        self.response = Some(response);
        self.status = "completed".to_string();
        self.completed_at = Some(chrono::Utc::now().timestamp());
        self.execution_time_ms = Some(execution_time_ms);
    }

    pub fn fail(&mut self, error: String) {
        self.status = "failed".to_string();
        self.error_details = Some(error);
        self.completed_at = Some(chrono::Utc::now().timestamp());
    }
}

// API Response types

/// Simplified session info for list views
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SessionListItem {
    pub id: i64,
    pub agent_name: String,
    pub user_prompt: String,
    pub status: String,
    pub started_at: i64,
}

/// List of sessions response
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionListItem>,
}

/// Detailed session with messages and tool calls
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SessionResponse {
    pub id: i64,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    #[ts(type = "any")]
    pub config: Option<serde_json::Value>,
    pub status: String,
    pub result: Option<String>,
    pub messages: Vec<SessionMessage>,
    pub tool_calls: Vec<SessionToolCall>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

/// Message in session response
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

/// Tool call in session response
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SessionToolCall {
    pub tool_name: String,
    #[ts(type = "any")]
    pub request: serde_json::Value,
    #[ts(type = "any")]
    pub response: Option<serde_json::Value>,
    pub status: String,
    pub execution_time_ms: Option<i64>,
}
