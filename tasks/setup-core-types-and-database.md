# Setup Core Types and Database Infrastructure

## Objective

Create the foundational data layer for dwata by implementing:
1. A `shared-types` crate for common type definitions
2. Database models for storing agent sessions, tool calls, and user prompts
3. SQL migrations for database schema management
4. OS-specific database path configuration

This follows the nocodo project structure where `nocodo-api` uses types from `shared-types` and database functionality from `nocodo-agents`.

## Background

The dwata project will store interaction data similar to nocodo:
- **Agent sessions**: Track when agents are invoked, their configuration, and execution status
- **User prompts**: Store user messages that initiate agent sessions
- **Tool/function calls**: Record all tool invocations during agent execution
- **Session messages**: Store conversation history between user and agents

Database location: OS data folder + `dwata/db.sqlite3`
- macOS: `~/Library/Application Support/dwata/db.sqlite3`
- Linux: `~/.local/share/dwata/db.sqlite3`
- Windows: `%LOCALAPPDATA%\dwata\db.sqlite3`

## Implementation Plan

### Phase 1: Create shared-types Crate

#### 1.1 Add shared-types to workspace

**File: `Cargo.toml`**

Update workspace members:
```toml
[workspace]
resolver = "2"
members = [
    "dwata-api",
    "shared-types",  # Add this
]
```

#### 1.2 Create shared-types crate structure

```bash
mkdir -p shared-types/src
```

**File: `shared-types/Cargo.toml`**

```toml
[package]
name = "shared-types"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
uuid.workspace = true
```

#### 1.3 Define core types

**File: `shared-types/src/lib.rs`**

```rust
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub mod session;

pub use session::{
    AgentSession, AgentMessage, AgentToolCall, SessionListItem,
    SessionListResponse, SessionResponse, SessionMessage, SessionToolCall,
};

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
```

**File: `shared-types/src/session.rs`**

```rust
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListItem {
    pub id: i64,
    pub agent_name: String,
    pub user_prompt: String,
    pub status: String,
    pub started_at: i64,
}

/// List of sessions response
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionListItem>,
}

/// Detailed session with messages and tool calls
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub id: i64,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub config: Option<serde_json::Value>,
    pub status: String,
    pub result: Option<String>,
    pub messages: Vec<SessionMessage>,
    pub tool_calls: Vec<SessionToolCall>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

/// Message in session response
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

/// Tool call in session response
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionToolCall {
    pub tool_name: String,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub status: String,
    pub execution_time_ms: Option<i64>,
}
```

### Phase 2: Create Database Module in dwata-api

#### 2.1 Update dwata-api dependencies

**File: `dwata-api/Cargo.toml`**

Add dependencies:
```toml
[dependencies]
# ... existing dependencies
shared-types = { path = "../shared-types" }
rusqlite = { version = "0.37", features = ["bundled"] }
chrono.workspace = true
uuid.workspace = true
anyhow.workspace = true
home = "0.5"
```

#### 2.2 Create database module structure

```bash
mkdir -p dwata-api/src/database
```

**File: `dwata-api/src/database/mod.rs`**

```rust
pub mod migrations;
pub mod models;
pub mod queries;

use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

pub struct Database {
    pub connection: DbConnection,
}

impl Database {
    /// Create a new database connection and run migrations
    pub fn new(db_path: &PathBuf) -> anyhow::Result<Self> {
        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;

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

        conn.execute(
            "INSERT INTO agent_sessions (agent_name, provider, model, system_prompt, user_prompt, config, started_at, status)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'running')",
            params![agent_name, provider, model, system_prompt, user_prompt, config_json, now],
        )?;

        Ok(conn.last_insert_rowid())
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

        conn.execute(
            "INSERT INTO agent_messages (session_id, role, content, created_at)
                VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role, content, now],
        )?;

        Ok(conn.last_insert_rowid())
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

        conn.execute(
            "INSERT INTO agent_tool_calls (session_id, message_id, tool_call_id, tool_name, request, created_at, status)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending')",
            params![session_id, message_id, tool_call_id, tool_name, request_json, now],
        )?;

        Ok(conn.last_insert_rowid())
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
```

**File: `dwata-api/src/database/migrations.rs`**

```rust
use rusqlite::Connection;

/// Run all database migrations
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Create agent_sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_name TEXT NOT NULL,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            system_prompt TEXT,
            user_prompt TEXT NOT NULL,
            config TEXT,
            status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed')),
            started_at INTEGER NOT NULL,
            ended_at INTEGER,
            result TEXT,
            error TEXT
        )",
        [],
    )?;

    // Create agent_messages table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create agent_tool_calls table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_tool_calls (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            message_id INTEGER,
            tool_call_id TEXT NOT NULL,
            tool_name TEXT NOT NULL,
            request TEXT NOT NULL,
            response TEXT,
            status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'executing', 'completed', 'failed')),
            created_at INTEGER NOT NULL,
            completed_at INTEGER,
            execution_time_ms INTEGER,
            error_details TEXT,
            FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE,
            FOREIGN KEY (message_id) REFERENCES agent_messages (id) ON DELETE SET NULL
        )",
        [],
    )?;

    // Create indexes for performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_messages_session_created
            ON agent_messages(session_id, created_at)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_session
            ON agent_tool_calls(session_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_status
            ON agent_tool_calls(session_id, status)",
        [],
    )?;

    Ok(())
}

/// Check if database tables exist
pub fn has_schema(conn: &Connection) -> anyhow::Result<bool> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='agent_sessions'")?;
    Ok(stmt.exists([])?)
}
```

**File: `dwata-api/src/database/models.rs`**

```rust
// Re-export shared types for convenience
pub use shared_types::{AgentSession, AgentMessage, AgentToolCall};
```

**File: `dwata-api/src/database/queries.rs`**

```rust
use rusqlite::{Connection, params};
use shared_types::{AgentSession, AgentMessage, AgentToolCall, SessionListItem};

/// Get all sessions ordered by most recent
pub fn get_all_sessions(conn: &Connection) -> anyhow::Result<Vec<SessionListItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, user_prompt, status, started_at
            FROM agent_sessions
            ORDER BY started_at DESC"
    )?;

    let sessions = stmt.query_map([], |row| {
        Ok(SessionListItem {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            user_prompt: row.get(2)?,
            status: row.get(3)?,
            started_at: row.get(4)?,
        })
    })?;

    let mut result = Vec::new();
    for session in sessions {
        result.push(session?);
    }

    Ok(result)
}

/// Get a single session by ID
pub fn get_session(conn: &Connection, session_id: i64) -> anyhow::Result<Option<AgentSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, provider, model, system_prompt, user_prompt, config, status, started_at, ended_at, result, error
            FROM agent_sessions
            WHERE id = ?1"
    )?;

    let session = stmt.query_row([session_id], |row| {
        let config_str: Option<String> = row.get(6)?;
        let config = config_str.and_then(|s| serde_json::from_str(&s).ok());

        Ok(AgentSession {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            provider: row.get(2)?,
            model: row.get(3)?,
            system_prompt: row.get(4)?,
            user_prompt: row.get(5)?,
            config,
            status: row.get(7)?,
            started_at: row.get(8)?,
            ended_at: row.get(9)?,
            result: row.get(10)?,
            error: row.get(11)?,
        })
    });

    match session {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Get all messages for a session
pub fn get_session_messages(conn: &Connection, session_id: i64) -> anyhow::Result<Vec<AgentMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, role, content, created_at
            FROM agent_messages
            WHERE session_id = ?1
            ORDER BY created_at ASC"
    )?;

    let messages = stmt.query_map([session_id], |row| {
        Ok(AgentMessage {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;

    let mut result = Vec::new();
    for message in messages {
        result.push(message?);
    }

    Ok(result)
}

/// Get all tool calls for a session
pub fn get_session_tool_calls(conn: &Connection, session_id: i64) -> anyhow::Result<Vec<AgentToolCall>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, message_id, tool_call_id, tool_name, request, response, status, created_at, completed_at, execution_time_ms, error_details
            FROM agent_tool_calls
            WHERE session_id = ?1
            ORDER BY created_at ASC"
    )?;

    let calls = stmt.query_map([session_id], |row| {
        let request_str: String = row.get(5)?;
        let request = serde_json::from_str(&request_str).map_err(|_| {
            rusqlite::Error::InvalidColumnType(5, request_str.clone(), rusqlite::types::Type::Text)
        })?;

        let response: Option<serde_json::Value> = row.get(6).ok()
            .and_then(|s: String| serde_json::from_str(&s).ok());

        Ok(AgentToolCall {
            id: row.get(0)?,
            session_id: row.get(1)?,
            message_id: row.get(2)?,
            tool_call_id: row.get(3)?,
            tool_name: row.get(4)?,
            request,
            response,
            status: row.get(7)?,
            created_at: row.get(8)?,
            completed_at: row.get(9)?,
            execution_time_ms: row.get(10)?,
            error_details: row.get(11)?,
        })
    })?;

    let mut result = Vec::new();
    for call in calls {
        result.push(call?);
    }

    Ok(result)
}
```

#### 2.3 Create database path helper

**File: `dwata-api/src/helpers/mod.rs`**

```rust
pub mod database;
```

**File: `dwata-api/src/helpers/database.rs`**

```rust
use std::path::PathBuf;

/// Returns the path to the dwata database based on the operating system
///
/// # Returns
///
/// A PathBuf pointing to the database file
///
/// # Platform-specific paths
///
/// - **macOS**: `~/Library/Application Support/dwata/db.sqlite3`
/// - **Linux**: `~/.local/share/dwata/db.sqlite3`
/// - **Windows**: `%LOCALAPPDATA%\dwata\db.sqlite3`
pub fn get_db_path() -> anyhow::Result<PathBuf> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let db_path = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/dwata/db.sqlite3")
    } else if cfg!(target_os = "linux") {
        home.join(".local/share/dwata/db.sqlite3")
    } else if cfg!(windows) {
        home.join("AppData/Local/dwata/db.sqlite3")
    } else {
        anyhow::bail!("Unsupported operating system");
    };

    Ok(db_path)
}

/// Initialize the database connection
pub fn initialize_database() -> anyhow::Result<std::sync::Arc<crate::database::Database>> {
    let db_path = get_db_path()?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = crate::database::Database::new(&db_path)?;
    Ok(std::sync::Arc::new(db))
}
```

#### 2.4 Update dwata-api main.rs structure

**File: `dwata-api/src/lib.rs`**

```rust
pub mod database;
pub mod helpers;

pub use database::Database;
```

### Phase 3: Integration and Testing

#### 3.1 Update main.rs to initialize database

**File: `dwata-api/src/main.rs`**

Update to initialize database on startup:

```rust
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use std::sync::Arc;

mod database;
mod helpers;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Hello World"
    }))
}

#[get("/health")]
async fn health(db: web::Data<Arc<database::Database>>) -> impl Responder {
    // Test database connection
    match db.connection.lock() {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "status": "healthy",
            "database": "connected"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "unhealthy",
            "database": "disconnected"
        })),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize database
    let db = helpers::database::initialize_database()
        .expect("Failed to initialize database");

    println!("Database initialized at: {:?}", helpers::database::get_db_path().unwrap());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .service(hello)
            .service(health)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

#### 3.2 Build and test

```bash
# Build the project
cargo build

# Run the API server
cargo run --bin dwata-api

# Test the health endpoint
curl http://localhost:8080/health
```

#### 3.3 Verify database creation

After running the server, verify the database file exists:

```bash
# macOS
ls -la ~/Library/Application\ Support/dwata/

# Linux
ls -la ~/.local/share/dwata/

# Should see db.sqlite3 file
```

## Database Schema

### Tables

#### agent_sessions
- `id`: INTEGER PRIMARY KEY AUTOINCREMENT
- `agent_name`: TEXT NOT NULL
- `provider`: TEXT NOT NULL (e.g., "anthropic", "openai")
- `model`: TEXT NOT NULL (e.g., "claude-3-5-sonnet-20241022")
- `system_prompt`: TEXT (optional)
- `user_prompt`: TEXT NOT NULL
- `config`: TEXT (JSON string, optional agent-specific config)
- `status`: TEXT NOT NULL ('running', 'completed', 'failed')
- `started_at`: INTEGER NOT NULL (Unix timestamp)
- `ended_at`: INTEGER (Unix timestamp, optional)
- `result`: TEXT (optional final result)
- `error`: TEXT (optional error message)

#### agent_messages
- `id`: INTEGER PRIMARY KEY AUTOINCREMENT
- `session_id`: INTEGER NOT NULL (FK to agent_sessions)
- `role`: TEXT NOT NULL ('user', 'assistant', 'system', 'tool')
- `content`: TEXT NOT NULL
- `created_at`: INTEGER NOT NULL (Unix timestamp)

#### agent_tool_calls
- `id`: INTEGER PRIMARY KEY AUTOINCREMENT
- `session_id`: INTEGER NOT NULL (FK to agent_sessions)
- `message_id`: INTEGER (FK to agent_messages, optional)
- `tool_call_id`: TEXT NOT NULL (unique identifier from LLM)
- `tool_name`: TEXT NOT NULL
- `request`: TEXT NOT NULL (JSON string)
- `response`: TEXT (JSON string, optional)
- `status`: TEXT NOT NULL ('pending', 'executing', 'completed', 'failed')
- `created_at`: INTEGER NOT NULL (Unix timestamp)
- `completed_at`: INTEGER (Unix timestamp, optional)
- `execution_time_ms`: INTEGER (optional)
- `error_details`: TEXT (optional)

### Indexes

- `idx_agent_messages_session_created`: ON agent_messages(session_id, created_at)
- `idx_agent_tool_calls_session`: ON agent_tool_calls(session_id)
- `idx_agent_tool_calls_status`: ON agent_tool_calls(session_id, status)

## Next Steps (Future Tasks)

1. **API Endpoints**: Create REST endpoints for:
   - POST `/sessions` - Create new agent session
   - GET `/sessions` - List all sessions
   - GET `/sessions/:id` - Get session details with messages and tool calls
   - GET `/sessions/:id/messages` - Get messages for a session
   - GET `/sessions/:id/tool-calls` - Get tool calls for a session

2. **GUI Integration**: Update dwata GUI to:
   - Send user prompts to dwata-api
   - Display session history
   - Show real-time tool execution
   - Stream agent responses

3. **Agent Execution**: Implement agent execution logic using nocodo-agents and nocodo-llm-sdk

## References

- nocodo shared-types: `/Users/brainless/Projects/nocodo/shared-types/`
- nocodo-agents database: `/Users/brainless/Projects/nocodo/nocodo-agents/src/database/`
- nocodo-api integration: `/Users/brainless/Projects/nocodo/nocodo-api/src/helpers/`
