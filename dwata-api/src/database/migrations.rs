use duckdb::Connection;

/// Run all database migrations
#[allow(dead_code)]
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Create agent_sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_sessions (
            id BIGINT PRIMARY KEY,
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
            id BIGINT PRIMARY KEY,
            session_id BIGINT NOT NULL,
            role VARCHAR NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
            content VARCHAR NOT NULL,
            created_at BIGINT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions (id)
        )",
        [],
    )?;

    // Create agent_tool_calls table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_tool_calls (
            id BIGINT PRIMARY KEY,
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
            FOREIGN KEY (session_id) REFERENCES agent_sessions (id),
            FOREIGN KEY (message_id) REFERENCES agent_messages (id)
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

    // Create credentials_metadata table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS credentials_metadata (
            id INTEGER PRIMARY KEY,
            credential_type VARCHAR NOT NULL,
            identifier VARCHAR NOT NULL UNIQUE,
            username VARCHAR NOT NULL,
            service_name VARCHAR,
            port INTEGER,
            use_tls BOOLEAN DEFAULT TRUE,
            notes VARCHAR,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            last_accessed_at BIGINT,
            is_active BOOLEAN DEFAULT TRUE,
            extra_metadata VARCHAR
        )",
        [],
    )?;

    // Index for efficient listing and filtering
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_credentials_type_active
            ON credentials_metadata(credential_type, is_active)",
        [],
    )?;

    // Create download_jobs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS download_jobs (
            id VARCHAR PRIMARY KEY,
            source_type VARCHAR NOT NULL,
            credential_id VARCHAR NOT NULL,
            status VARCHAR NOT NULL DEFAULT 'pending',
            total_items BIGINT NOT NULL DEFAULT 0,
            downloaded_items BIGINT NOT NULL DEFAULT 0,
            failed_items BIGINT NOT NULL DEFAULT 0,
            skipped_items BIGINT NOT NULL DEFAULT 0,
            in_progress_items BIGINT NOT NULL DEFAULT 0,
            bytes_downloaded BIGINT NOT NULL DEFAULT 0,
            source_state VARCHAR NOT NULL,
            error_message VARCHAR,
            retry_count INTEGER DEFAULT 0,
            created_at BIGINT NOT NULL,
            started_at BIGINT,
            updated_at BIGINT NOT NULL,
            completed_at BIGINT,
            last_sync_at BIGINT,
            FOREIGN KEY (credential_id) REFERENCES credentials_metadata (id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_download_jobs_status
            ON download_jobs(status, updated_at)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_download_jobs_credential
            ON download_jobs(credential_id)",
        [],
    )?;

    // Create download_items table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS download_items (
            id VARCHAR PRIMARY KEY,
            job_id VARCHAR NOT NULL,
            source_identifier VARCHAR NOT NULL,
            source_folder VARCHAR,
            item_type VARCHAR NOT NULL,
            status VARCHAR NOT NULL,
            size_bytes BIGINT,
            mime_type VARCHAR,
            metadata VARCHAR,
            error_message VARCHAR,
            retry_count INTEGER DEFAULT 0,
            last_attempt_at BIGINT,
            local_path VARCHAR,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            downloaded_at BIGINT,
            FOREIGN KEY (job_id) REFERENCES download_jobs (id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_download_items_job_status
            ON download_items(job_id, status)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_download_items_source_identifier
            ON download_items(job_id, source_identifier)",
        [],
    )?;

    Ok(())
}

/// Check if database tables exist
#[allow(dead_code)]
pub fn has_schema(conn: &Connection) -> anyhow::Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT table_name FROM information_schema.tables WHERE table_name='agent_sessions'",
    )?;
    Ok(stmt.exists([])?)
}
