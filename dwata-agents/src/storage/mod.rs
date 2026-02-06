pub mod sqlite_storage;

pub use sqlite_storage::SqliteAgentStorage;

use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub agent_type: String,
    pub objective: String,
    pub context_data: Option<String>,
    pub status: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<i64>,
    pub session_id: i64,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<i64>,
    pub session_id: i64,
    pub tool_name: String,
    pub tool_input: String,
    pub tool_output: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait AgentStorage: Send + Sync {
    async fn create_session(&self, session: Session) -> Result<i64>;
    async fn get_session(&self, session_id: i64) -> Result<Option<Session>>;
    async fn update_session(&self, session: Session) -> Result<()>;

    async fn create_message(&self, message: Message) -> Result<i64>;
    async fn get_messages(&self, session_id: i64) -> Result<Vec<Message>>;

    async fn create_tool_call(&self, tool_call: ToolCall) -> Result<i64>;
    async fn update_tool_call(&self, tool_call: ToolCall) -> Result<()>;
    async fn get_tool_calls(&self, session_id: i64) -> Result<Vec<ToolCall>>;
}
