pub mod credentials;
pub mod migrations;
pub mod models;
pub mod queries;

use duckdb::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

pub struct Database {
    pub connection: DbConnection,
}

#[allow(dead_code)]
impl Database {
    /// Create a new database connection and run migrations
    pub fn new(db_path: &PathBuf) -> anyhow::Result<Self> {
        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        let database = Database {
            connection: Arc::new(Mutex::new(conn)),
        };

        database.run_migrations()?;
        Ok(database)
    }

    fn run_migrations(&self) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        migrations::run_migrations(&conn)
    }

    // Session management
    pub fn create_session(
        &self,
        agent_name: &str,
        provider: &str,
        model: &str,
        system_prompt: Option<&str>,
        user_prompt: &str,
        config: Option<serde_json::Value>,
    ) -> anyhow::Result<i64> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        let config_json = config
            .map(|c| serde_json::to_string(&c).unwrap())
            .unwrap_or_else(|| "null".to_string());

        let id: i64 = conn.query_row(
            "INSERT INTO agent_sessions (agent_name, provider, model, system_prompt, user_prompt, config, started_at, status)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'running') RETURNING id",
            params![agent_name, provider, model, system_prompt, user_prompt, config_json, now],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    pub fn complete_session(&self, session_id: i64, result: &str) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_sessions SET status = 'completed', ended_at = ?1, result = ?2
                WHERE id = ?3",
            params![now, result, session_id],
        )?;

        Ok(())
    }

    pub fn fail_session(&self, session_id: i64, error: &str) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_sessions SET status = 'failed', ended_at = ?1, error = ?2
                WHERE id = ?3",
            params![now, error, session_id],
        )?;

        Ok(())
    }

    // Message management
    pub fn create_message(
        &self,
        session_id: i64,
        role: &str,
        content: &str,
    ) -> anyhow::Result<i64> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        let id: i64 = conn.query_row(
            "INSERT INTO agent_messages (session_id, role, content, created_at)
                VALUES (?1, ?2, ?3, ?4) RETURNING id",
            params![session_id, role, content, now],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    // Tool call management
    pub fn create_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call_id: &str,
        tool_name: &str,
        request: serde_json::Value,
    ) -> anyhow::Result<i64> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        let request_json = serde_json::to_string(&request)?;

        let id: i64 = conn.query_row(
            "INSERT INTO agent_tool_calls (session_id, message_id, tool_call_id, tool_name, request, created_at, status)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending') RETURNING id",
            params![session_id, message_id, tool_call_id, tool_name, request_json, now],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    pub fn complete_tool_call(
        &self,
        call_id: i64,
        response: serde_json::Value,
        execution_time_ms: i64,
    ) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        let response_json = serde_json::to_string(&response)?;

        conn.execute(
            "UPDATE agent_tool_calls SET status = 'completed', response = ?1, completed_at = ?2, execution_time_ms = ?3
                WHERE id = ?4",
            params![response_json, now, execution_time_ms, call_id],
        )?;

        Ok(())
    }

    pub fn fail_tool_call(&self, call_id: i64, error: &str) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_tool_calls SET status = 'failed', error_details = ?1, completed_at = ?2
                WHERE id = ?3",
            params![error, now, call_id],
        )?;

        Ok(())
    }
}
