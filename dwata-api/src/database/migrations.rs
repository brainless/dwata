use duckdb::Connection;

/// Run all database migrations
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Create agent_sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_sessions (
            id INTEGER PRIMARY KEY,
            agent_name VARCHAR NOT NULL,
            provider VARCHAR NOT NULL,
            model VARCHAR NOT NULL,
            system_prompt VARCHAR,
            user_prompt VARCHAR NOT NULL,
            config VARCHAR,
            status VARCHAR NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed')),
            started_at BIGINT NOT NULL,
            ended_at BIGINT,
            result VARCHAR,
            error VARCHAR
        )",
        [],
    )?;

    // Create agent_messages table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_messages (
            id INTEGER PRIMARY KEY,
            session_id BIGINT NOT NULL,
            role VARCHAR NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
            content VARCHAR NOT NULL,
            created_at BIGINT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create agent_tool_calls table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_tool_calls (
            id INTEGER PRIMARY KEY,
            session_id BIGINT NOT NULL,
            message_id BIGINT,
            tool_call_id VARCHAR NOT NULL,
            tool_name VARCHAR NOT NULL,
            request VARCHAR NOT NULL,
            response VARCHAR,
            status VARCHAR NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'executing', 'completed', 'failed')),
            created_at BIGINT NOT NULL,
            completed_at BIGINT,
            execution_time_ms BIGINT,
            error_details VARCHAR,
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
    let mut stmt = conn.prepare(
        "SELECT table_name FROM information_schema.tables WHERE table_name='agent_sessions'",
    )?;
    Ok(stmt.exists([])?)
}
