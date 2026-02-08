use super::{AgentStorage, Message, Session, ToolCall};
use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

pub struct SqliteAgentStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteAgentStorage {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl AgentStorage for SqliteAgentStorage {
    async fn create_session(&self, session: Session) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO agent_sessions
             (agent_name, provider, model, system_prompt, user_prompt, config, status, started_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                session.agent_type,
                "financial-extractor",
                "agent",
                None::<String>,
                session.objective,
                session.context_data.unwrap_or_else(|| "null".to_string()),
                session.status,
                now,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    async fn get_session(&self, session_id: i64) -> anyhow::Result<Option<Session>> {
        let conn = self.conn.lock().unwrap();

        let session = conn
            .query_row(
                "SELECT id, agent_name, user_prompt, config, status, result
                 FROM agent_sessions WHERE id = ?",
                [session_id],
                |row| {
                    Ok(Session {
                        id: Some(row.get(0)?),
                        agent_type: row.get(1)?,
                        objective: row.get(2)?,
                        context_data: row.get(3)?,
                        status: row.get(4)?,
                        result: row.get(5)?,
                    })
                },
            )
            .optional()?;

        Ok(session)
    }

    async fn update_session(&self, session: Session) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_sessions
             SET status = ?, result = ?, ended_at = ?
             WHERE id = ?",
            rusqlite::params![
                session.status,
                session.result,
                if session.status == "completed" || session.status == "failed" {
                    Some(now)
                } else {
                    None
                },
                session.id.unwrap(),
            ],
        )?;

        Ok(())
    }

    async fn create_message(&self, message: Message) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO agent_messages (session_id, role, content, created_at)
             VALUES (?, ?, ?, ?)",
            rusqlite::params![message.session_id, message.role, message.content, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    async fn get_messages(&self, session_id: i64) -> anyhow::Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content
             FROM agent_messages
             WHERE session_id = ?
             ORDER BY created_at ASC",
        )?;

        let messages = stmt
            .query_map([session_id], |row| {
                Ok(Message {
                    id: Some(row.get(0)?),
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    async fn create_tool_call(&self, tool_call: ToolCall) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO agent_tool_calls
             (session_id, tool_name, request, status, created_at)
             VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![
                tool_call.session_id,
                tool_call.tool_name,
                tool_call.tool_input,
                tool_call.status,
                now,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    async fn update_tool_call(&self, tool_call: ToolCall) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_tool_calls
             SET response = ?, status = ?, error_details = ?, completed_at = ?
             WHERE id = ?",
            rusqlite::params![
                tool_call.tool_output,
                tool_call.status,
                tool_call.error_message,
                now,
                tool_call.id.unwrap(),
            ],
        )?;

        Ok(())
    }

    async fn get_tool_calls(&self, session_id: i64) -> anyhow::Result<Vec<ToolCall>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, session_id, tool_name, request, response, status, error_details
             FROM agent_tool_calls
             WHERE session_id = ?
             ORDER BY created_at ASC",
        )?;

        let tool_calls = stmt
            .query_map([session_id], |row| {
                Ok(ToolCall {
                    id: Some(row.get(0)?),
                    session_id: row.get(1)?,
                    tool_name: row.get(2)?,
                    tool_input: row.get(3)?,
                    tool_output: row.get(4)?,
                    status: row.get(5)?,
                    error_message: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tool_calls)
    }
}
