use duckdb::Connection;

/// Run all database migrations
#[allow(dead_code)]
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Create sequences
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_agent_sessions_id", [])?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_agent_messages_id", [])?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_agent_tool_calls_id", [])?;
    conn.execute(
        "CREATE SEQUENCE IF NOT EXISTS seq_credentials_metadata_id",
        [],
    )?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_download_jobs_id", [])?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_download_items_id", [])?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_emails_id", [])?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_email_attachments_id", [])?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_extraction_jobs_id", [])?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_events_id", [])?;
    conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_contacts_id", [])?;

    // Create agent_sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_sessions (
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_agent_sessions_id'),
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
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_agent_messages_id'),
            session_id INTEGER NOT NULL,
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
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_agent_tool_calls_id'),
            session_id INTEGER NOT NULL,
            message_id INTEGER,
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
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_credentials_metadata_id'),
            credential_type VARCHAR NOT NULL,
            identifier VARCHAR NOT NULL UNIQUE,
            username VARCHAR NOT NULL,
            service_name VARCHAR,
            port INTEGER,
            use_tls BOOLEAN DEFAULT true,
            notes VARCHAR,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            last_accessed_at BIGINT,
            is_active BOOLEAN DEFAULT true,
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
    // Note: Foreign keys removed due to DuckDB constraint bugs with UPDATE operations
    // Referential integrity maintained through application logic
    conn.execute(
        "CREATE TABLE IF NOT EXISTS download_jobs (
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_download_jobs_id'),
            source_type VARCHAR NOT NULL,
            credential_id INTEGER NOT NULL,
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
            last_sync_at BIGINT
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
    // Note: Foreign keys removed due to DuckDB constraint bugs with UPDATE operations
    conn.execute(
        "CREATE TABLE IF NOT EXISTS download_items (
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_download_items_id'),
            job_id INTEGER NOT NULL,
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
            downloaded_at BIGINT
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

    // Create emails table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS emails (
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_emails_id'),
            download_item_id INTEGER,
            uid INTEGER NOT NULL,
            folder VARCHAR NOT NULL,
            message_id VARCHAR,
            subject VARCHAR,
            from_address VARCHAR NOT NULL,
            from_name VARCHAR,
            to_addresses VARCHAR,
            cc_addresses VARCHAR,
            bcc_addresses VARCHAR,
            reply_to VARCHAR,
            date_sent BIGINT,
            date_received BIGINT NOT NULL,
            body_text VARCHAR,
            body_html VARCHAR,
            is_read BOOLEAN DEFAULT false,
            is_flagged BOOLEAN DEFAULT false,
            is_draft BOOLEAN DEFAULT false,
            is_answered BOOLEAN DEFAULT false,
            has_attachments BOOLEAN DEFAULT false,
            attachment_count INTEGER DEFAULT 0,
            size_bytes INTEGER,
            thread_id VARCHAR,
            labels VARCHAR,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_emails_download_item ON emails(download_item_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_emails_folder_date ON emails(folder, date_received DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_emails_message_id ON emails(message_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_emails_from ON emails(from_address)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_emails_date_sent ON emails(date_sent DESC)",
        [],
    )?;

    // Create email_attachments table
    // Note: Foreign keys removed due to DuckDB constraint bugs with UPDATE operations
    conn.execute(
        "CREATE TABLE IF NOT EXISTS email_attachments (
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_email_attachments_id'),
            email_id INTEGER NOT NULL,
            filename VARCHAR NOT NULL,
            content_type VARCHAR,
            size_bytes INTEGER,
            content_id VARCHAR,
            file_path VARCHAR NOT NULL,
            checksum VARCHAR,
            is_inline BOOLEAN DEFAULT false,
            extraction_status VARCHAR DEFAULT 'pending',
            extracted_text VARCHAR,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_attachments_email ON email_attachments(email_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_attachments_checksum ON email_attachments(checksum)",
        [],
    )?;

    // Create extraction_jobs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS extraction_jobs (
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_extraction_jobs_id'),
            source_type VARCHAR NOT NULL,
            extractor_type VARCHAR NOT NULL,
            status VARCHAR NOT NULL DEFAULT 'pending',
            total_items INTEGER NOT NULL DEFAULT 0,
            processed_items INTEGER NOT NULL DEFAULT 0,
            extracted_entities INTEGER NOT NULL DEFAULT 0,
            failed_items INTEGER NOT NULL DEFAULT 0,
            source_config VARCHAR NOT NULL,
            events_extracted INTEGER NOT NULL DEFAULT 0,
            contacts_extracted INTEGER NOT NULL DEFAULT 0,
            error_message VARCHAR,
            created_at BIGINT NOT NULL,
            started_at BIGINT,
            updated_at BIGINT NOT NULL,
            completed_at BIGINT
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_extraction_jobs_status
            ON extraction_jobs(status, updated_at)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_extraction_jobs_extractor
            ON extraction_jobs(extractor_type)",
        [],
    )?;

    // Create events table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_events_id'),
            extraction_job_id INTEGER,
            email_id INTEGER,
            name VARCHAR NOT NULL,
            description VARCHAR,
            event_date BIGINT NOT NULL,
            location VARCHAR,
            attendees VARCHAR,
            confidence FLOAT,
            requires_review BOOLEAN DEFAULT false,
            is_confirmed BOOLEAN DEFAULT false,
            project_id INTEGER,
            task_id INTEGER,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_date ON events(event_date DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_extraction_job ON events(extraction_job_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_email ON events(email_id)",
        [],
    )?;

    // Create contacts table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS contacts (
            id INTEGER PRIMARY KEY DEFAULT nextval('seq_contacts_id'),
            extraction_job_id INTEGER,
            email_id INTEGER,
            name VARCHAR NOT NULL,
            email VARCHAR,
            phone VARCHAR,
            organization VARCHAR,
            confidence FLOAT,
            requires_review BOOLEAN DEFAULT false,
            is_confirmed BOOLEAN DEFAULT false,
            is_duplicate BOOLEAN DEFAULT false,
            merged_into_contact_id INTEGER,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            UNIQUE(email)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_contacts_extraction_job ON contacts(extraction_job_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_contacts_email ON contacts(email)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_contacts_name ON contacts(name)",
        [],
    )?;

    // Migration: Drop foreign key constraints that cause DuckDB issues
    // Note: DuckDB doesn't support ALTER TABLE DROP CONSTRAINT directly
    // So we need to recreate tables without foreign keys if they exist with them
    // For now, just log a warning - user should delete db.duckdb to get fresh schema
    tracing::info!("Foreign key constraints removed from schema for new installations");

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
