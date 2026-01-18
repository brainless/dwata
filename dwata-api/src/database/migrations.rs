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
