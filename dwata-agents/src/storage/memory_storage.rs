use super::{AgentStorage, Message, Session, ToolCall};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct MemoryState {
    next_session_id: i64,
    next_message_id: i64,
    next_tool_call_id: i64,
    sessions: HashMap<i64, Session>,
    messages: Vec<Message>,
    tool_calls: Vec<ToolCall>,
}

pub struct InMemoryAgentStorage {
    state: Arc<Mutex<MemoryState>>,
}

impl InMemoryAgentStorage {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryState::default())),
        }
    }
}

#[async_trait]
impl AgentStorage for InMemoryAgentStorage {
    async fn create_session(&self, mut session: Session) -> anyhow::Result<i64> {
        let mut state = self.state.lock().unwrap();
        state.next_session_id += 1;
        let id = state.next_session_id;
        session.id = Some(id);
        state.sessions.insert(id, session);
        Ok(id)
    }

    async fn get_session(&self, session_id: i64) -> anyhow::Result<Option<Session>> {
        let state = self.state.lock().unwrap();
        Ok(state.sessions.get(&session_id).cloned())
    }

    async fn update_session(&self, session: Session) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        if let Some(id) = session.id {
            state.sessions.insert(id, session);
        }
        Ok(())
    }

    async fn create_message(&self, mut message: Message) -> anyhow::Result<i64> {
        let mut state = self.state.lock().unwrap();
        state.next_message_id += 1;
        let id = state.next_message_id;
        message.id = Some(id);
        state.messages.push(message);
        Ok(id)
    }

    async fn get_messages(&self, session_id: i64) -> anyhow::Result<Vec<Message>> {
        let state = self.state.lock().unwrap();
        Ok(state
            .messages
            .iter()
            .filter(|m| m.session_id == session_id)
            .cloned()
            .collect())
    }

    async fn create_tool_call(&self, mut tool_call: ToolCall) -> anyhow::Result<i64> {
        let mut state = self.state.lock().unwrap();
        state.next_tool_call_id += 1;
        let id = state.next_tool_call_id;
        tool_call.id = Some(id);
        state.tool_calls.push(tool_call);
        Ok(id)
    }

    async fn update_tool_call(&self, tool_call: ToolCall) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        if let Some(id) = tool_call.id {
            if let Some(existing) = state.tool_calls.iter_mut().find(|t| t.id == Some(id)) {
                *existing = tool_call;
            }
        }
        Ok(())
    }

    async fn get_tool_calls(&self, session_id: i64) -> anyhow::Result<Vec<ToolCall>> {
        let state = self.state.lock().unwrap();
        Ok(state
            .tool_calls
            .iter()
            .filter(|t| t.session_id == session_id)
            .cloned()
            .collect())
    }
}
