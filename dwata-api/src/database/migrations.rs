use rusqlite::Connection;

fn table_has_column(conn: &Connection, table: &str, column: &str) -> anyhow::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn table_sql_contains(conn: &Connection, table: &str, needle: &str) -> anyhow::Result<bool> {
    let sql: Option<String> = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get(0),
    )?;
    Ok(sql.map(|s| s.contains(needle)).unwrap_or(false))
}

fn rebuild_financial_patterns_table(conn: &mut Connection) -> anyhow::Result<()> {
    let tx = conn.transaction()?;

    tx.execute(
        "ALTER TABLE financial_patterns RENAME TO financial_patterns_old",
        [],
    )?;

    let has_currency_group = table_has_column(&tx, "financial_patterns_old", "currency_group")?;

    tx.execute(
        "CREATE TABLE financial_patterns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,

            -- Pattern identity
            name VARCHAR NOT NULL,
            regex_pattern VARCHAR NOT NULL,
            description VARCHAR,
            sender_email VARCHAR,

            -- Pattern metadata
            document_type VARCHAR NOT NULL,
            status VARCHAR NOT NULL,

            -- Capture group indices (which regex group contains each field)
            amount_group INTEGER NOT NULL,
            vendor_group INTEGER,
            source_vendor_group INTEGER,
            destination_vendor_group INTEGER,
            date_group INTEGER,
            reference_group INTEGER,
            currency_group INTEGER,

            -- Management flags
            is_default BOOLEAN DEFAULT false,
            is_active BOOLEAN DEFAULT true,

            -- Usage statistics
            match_count INTEGER DEFAULT 0,
            last_matched_at BIGINT,

            -- Timestamps
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,

            -- Uniqueness constraints
            UNIQUE(name)
        )",
        [],
    )?;

    let currency_select = if has_currency_group {
        "currency_group"
    } else {
        "NULL as currency_group"
    };

    tx.execute(
        &format!(
            "INSERT INTO financial_patterns (
                id, name, regex_pattern, description, sender_email, document_type, status,
                amount_group, vendor_group, source_vendor_group, destination_vendor_group, date_group,
                reference_group, currency_group, is_default, is_active, match_count, last_matched_at, created_at, updated_at
            )
            SELECT
                id, name, regex_pattern, description, sender_email, document_type, status,
                amount_group, vendor_group, source_vendor_group, destination_vendor_group, date_group,
                reference_group, {}, is_default, is_active, match_count, last_matched_at, created_at, updated_at
            FROM financial_patterns_old",
            currency_select
        ),
        [],
    )?;

    tx.execute("DROP TABLE financial_patterns_old", [])?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_patterns_active ON financial_patterns(is_active)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_patterns_type ON financial_patterns(document_type)",
        [],
    )?;
    tx.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_financial_patterns_regex_sender_unique
         ON financial_patterns(regex_pattern, sender_email)
         WHERE sender_email IS NOT NULL",
        [],
    )?;
    tx.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_financial_patterns_regex_unique_null_sender
         ON financial_patterns(regex_pattern)
         WHERE sender_email IS NULL",
        [],
    )?;

    tx.commit()?;
    Ok(())
}

fn migrate_financial_schema(conn: &Connection) -> anyhow::Result<()> {
    let has_data_source_type =
        table_has_column(conn, "financial_transactions", "data_source_type")?;
    if !has_data_source_type {
        conn.execute("DROP TABLE IF EXISTS financial_transactions", [])?;
    }

    let has_data_source_type =
        table_has_column(conn, "financial_extraction_sources", "data_source_type")?;
    if !has_data_source_type {
        conn.execute("DROP TABLE IF EXISTS financial_extraction_sources", [])?;
    }

    Ok(())
}

fn make_transaction_description_nullable(conn: &mut Connection) -> anyhow::Result<()> {
    if table_sql_contains(
        conn,
        "financial_transactions",
        "description VARCHAR NOT NULL",
    )? {
        tracing::info!("Making financial_transactions.description nullable");

        let tx = conn.transaction()?;

        // SQLite requires table rebuild to change column constraints
        tx.execute(
            "ALTER TABLE financial_transactions RENAME TO financial_transactions_old",
            [],
        )?;

        // Create new table with nullable description
        tx.execute(
            "CREATE TABLE financial_transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                data_source_type VARCHAR NOT NULL,
                data_source_id VARCHAR NOT NULL,
                extraction_job_id INTEGER,
                document_type VARCHAR NOT NULL,
                description VARCHAR,
                amount DOUBLE NOT NULL,
                currency VARCHAR NOT NULL DEFAULT 'USD',
                transaction_date VARCHAR NOT NULL,
                category VARCHAR,
                vendor VARCHAR,
                source_vendor_id INTEGER,
                destination_vendor_id INTEGER,
                status VARCHAR NOT NULL,
                source_file VARCHAR,
                confidence DOUBLE,
                requires_review BOOLEAN DEFAULT false,
                extracted_at BIGINT NOT NULL,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL,
                notes VARCHAR,
                transaction_reference VARCHAR,
                UNIQUE(data_source_type, data_source_id, transaction_reference),
                UNIQUE(data_source_type, data_source_id, amount, vendor, transaction_date, document_type),
                FOREIGN KEY (source_vendor_id) REFERENCES transaction_vendors (id),
                FOREIGN KEY (destination_vendor_id) REFERENCES transaction_vendors (id)
            )",
            [],
        )?;

        // Copy all data
        tx.execute(
            "INSERT INTO financial_transactions SELECT * FROM financial_transactions_old",
            [],
        )?;
        tx.execute("DROP TABLE financial_transactions_old", [])?;

        // Recreate indexes
        tx.execute("CREATE INDEX IF NOT EXISTS idx_financial_transactions_data_source ON financial_transactions(data_source_type, data_source_id)", [])?;
        tx.execute("CREATE INDEX IF NOT EXISTS idx_financial_transactions_date ON financial_transactions(transaction_date DESC)", [])?;
        tx.execute("CREATE INDEX IF NOT EXISTS idx_financial_transactions_vendor ON financial_transactions(vendor)", [])?;
        tx.execute("CREATE INDEX IF NOT EXISTS idx_financial_transactions_source_vendor ON financial_transactions(source_vendor_id)", [])?;
        tx.execute("CREATE INDEX IF NOT EXISTS idx_financial_transactions_destination_vendor ON financial_transactions(destination_vendor_id)", [])?;
        tx.execute("CREATE INDEX IF NOT EXISTS idx_financial_transactions_reference ON financial_transactions(transaction_reference)", [])?;

        tx.commit()?;
        tracing::info!("Successfully made description nullable");
    }
    Ok(())
}

/// Run all database migrations
#[allow(dead_code)]
pub fn run_migrations(conn: &mut Connection) -> anyhow::Result<()> {
    // Create agent_sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS download_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_type VARCHAR NOT NULL,
            credential_id INTEGER NOT NULL,
            job_type VARCHAR NOT NULL DEFAULT 'recent-sync',
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

    // Migration: Add job_type column to download_jobs if it doesn't exist
    let has_job_type: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('download_jobs') WHERE name='job_type'",
        [],
        |row| {
            let count: i64 = row.get(0)?;
            Ok(count > 0)
        },
    )?;

    if !has_job_type {
        tracing::info!("Adding job_type column to download_jobs table");
        conn.execute(
            "ALTER TABLE download_jobs ADD COLUMN job_type VARCHAR NOT NULL DEFAULT 'recent-sync'",
            [],
        )?;
    }

    // Create download_items table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS download_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            download_item_id INTEGER,
            credential_id INTEGER NOT NULL,
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
        "CREATE INDEX IF NOT EXISTS idx_emails_credential ON emails(credential_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_emails_credential_date
            ON emails(credential_id, date_received DESC)",
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS email_attachments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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

    // Create document_sources table (source-level state for unified documents)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS document_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_type VARCHAR NOT NULL CHECK (source_type IN ('imap-account', 'local-folder', 'cloud-drive', 'cloud-mailbox', 'manual-import')),
            display_name VARCHAR NOT NULL,
            credential_id INTEGER,
            root_reference VARCHAR,
            access_state VARCHAR NOT NULL DEFAULT 'unknown'
                CHECK (access_state IN ('accessible', 'offline', 'unreachable', 'disabled', 'unknown')),
            permission_state VARCHAR NOT NULL DEFAULT 'unknown'
                CHECK (permission_state IN ('granted', 'expired', 'revoked', 'insufficient-scope', 'forbidden', 'unknown')),
            access_checked_at BIGINT,
            permission_checked_at BIGINT,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_document_sources_type
            ON document_sources(source_type)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_document_sources_credential
            ON document_sources(credential_id)",
        [],
    )?;

    // Create documents table (unified browse/search identity for emails, attachments, and files)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL,
            kind VARCHAR NOT NULL CHECK (kind IN ('email', 'attachment', 'file')),
            parent_document_id INTEGER,
            email_id INTEGER,
            attachment_id INTEGER,
            title VARCHAR,
            canonical_name VARCHAR,
            mime_type VARCHAR,
            size_bytes BIGINT,
            checksum_sha256 VARCHAR,
            storage_path VARCHAR,
            external_uri VARCHAR,
            date_created BIGINT,
            date_modified BIGINT,
            date_received BIGINT,
            indexed_at BIGINT,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            FOREIGN KEY (source_id) REFERENCES document_sources(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_document_id) REFERENCES documents(id) ON DELETE SET NULL,
            FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE SET NULL,
            FOREIGN KEY (attachment_id) REFERENCES email_attachments(id) ON DELETE SET NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_source_kind
            ON documents(source_id, kind)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_parent
            ON documents(parent_document_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_email_id
            ON documents(email_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_attachment_id
            ON documents(attachment_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_received_date
            ON documents(date_received DESC, id DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_modified_date
            ON documents(date_modified DESC, id DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_created_date
            ON documents(created_at DESC, id DESC)",
        [],
    )?;

    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_source_external_uri_unique
         ON documents(source_id, external_uri)
         WHERE external_uri IS NOT NULL",
        [],
    )?;

    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_source_storage_path_unique
         ON documents(source_id, storage_path)
         WHERE storage_path IS NOT NULL",
        [],
    )?;

    // Create extraction_jobs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS extraction_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            companies_extracted INTEGER NOT NULL DEFAULT 0,
            positions_extracted INTEGER NOT NULL DEFAULT 0,
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
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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

    // Create companies table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS companies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            extraction_job_id INTEGER,
            name VARCHAR NOT NULL,
            description VARCHAR,
            industry VARCHAR,
            location VARCHAR,
            website VARCHAR,
            linkedin_url VARCHAR,
            is_duplicate BOOLEAN DEFAULT false,
            merged_into_company_id INTEGER,
            confidence FLOAT,
            requires_review BOOLEAN DEFAULT false,
            is_confirmed BOOLEAN DEFAULT false,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            UNIQUE(name, location)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_companies_name ON companies(name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_companies_extraction_job ON companies(extraction_job_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_companies_linkedin_url ON companies(linkedin_url)",
        [],
    )?;

    // Create contact_links table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS contact_links (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            contact_id INTEGER NOT NULL,
            link_type VARCHAR NOT NULL,
            url VARCHAR NOT NULL,
            label VARCHAR,
            is_primary BOOLEAN DEFAULT false,
            is_verified BOOLEAN DEFAULT false,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            UNIQUE(contact_id, link_type, url)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_contact_links_contact ON contact_links(contact_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_contact_links_type ON contact_links(link_type)",
        [],
    )?;

    // Create positions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS positions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            extraction_job_id INTEGER,
            contact_id INTEGER NOT NULL,
            company_id INTEGER NOT NULL,
            title VARCHAR NOT NULL,
            description VARCHAR,
            location VARCHAR,
            started_on VARCHAR,
            finished_on VARCHAR,
            started_date BIGINT,
            finished_date BIGINT,
            is_current BOOLEAN DEFAULT false,
            confidence FLOAT,
            requires_review BOOLEAN DEFAULT false,
            is_confirmed BOOLEAN DEFAULT false,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_positions_contact ON positions(contact_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_positions_company ON positions(company_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_positions_extraction_job ON positions(extraction_job_id)",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_positions_dates ON positions(started_date DESC, finished_date DESC)", [])?;

    // Create linkedin_connections table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS linkedin_connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            extraction_job_id INTEGER NOT NULL,
            contact_id INTEGER NOT NULL,
            connected_on VARCHAR,
            connected_date BIGINT,
            connection_source VARCHAR,
            direction VARCHAR,
            invitation_message VARCHAR,
            invitation_sent_at VARCHAR,
            company_at_connection VARCHAR,
            position_at_connection VARCHAR,
            created_at BIGINT NOT NULL,
            UNIQUE(contact_id, extraction_job_id)
        )",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_linkedin_connections_contact ON linkedin_connections(contact_id)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_linkedin_connections_extraction_job ON linkedin_connections(extraction_job_id)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_linkedin_connections_date ON linkedin_connections(connected_date DESC)", [])?;

    migrate_financial_schema(conn)?;
    make_transaction_description_nullable(&mut *conn)?;

    // Create transaction_vendors table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS transaction_vendors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vendor_type VARCHAR NOT NULL,
            vendor_name VARCHAR NOT NULL,
            vendor_external_id VARCHAR,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            UNIQUE(vendor_type, vendor_name, vendor_external_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_transaction_vendors_name ON transaction_vendors(vendor_name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_transaction_vendors_type ON transaction_vendors(vendor_type)",
        [],
    )?;

    // Create financial_transactions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS financial_transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,

            -- Source tracking (agnostic to source type)
            data_source_type VARCHAR NOT NULL,
            data_source_id VARCHAR NOT NULL,
            extraction_job_id INTEGER,

            -- Transaction data
            document_type VARCHAR NOT NULL,
            description VARCHAR,
            amount DOUBLE NOT NULL,
            currency VARCHAR NOT NULL DEFAULT 'USD',
            transaction_date VARCHAR NOT NULL,

            -- Additional fields
            category VARCHAR,
            vendor VARCHAR,
            source_vendor_id INTEGER,
            destination_vendor_id INTEGER,
            status VARCHAR NOT NULL,

            -- Metadata
            source_file VARCHAR,
            confidence DOUBLE,
            requires_review BOOLEAN DEFAULT false,

            -- Timestamps
            extracted_at BIGINT NOT NULL,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,

            notes VARCHAR,
            transaction_reference VARCHAR,
            UNIQUE(data_source_type, data_source_id, transaction_reference),
            UNIQUE(data_source_type, data_source_id, amount, vendor, transaction_date, document_type),
            FOREIGN KEY (source_vendor_id) REFERENCES transaction_vendors (id),
            FOREIGN KEY (destination_vendor_id) REFERENCES transaction_vendors (id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_transactions_data_source ON financial_transactions(data_source_type, data_source_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_transactions_date ON financial_transactions(transaction_date DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_transactions_vendor ON financial_transactions(vendor)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_transactions_source_vendor ON financial_transactions(source_vendor_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_transactions_destination_vendor ON financial_transactions(destination_vendor_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_transactions_reference ON financial_transactions(transaction_reference)",
        [],
    )?;

    // Track which sources have been processed for financial extraction
    conn.execute(
        "CREATE TABLE IF NOT EXISTS financial_extraction_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            data_source_type VARCHAR NOT NULL,
            data_source_id VARCHAR NOT NULL,
            extraction_job_id INTEGER,
            extracted_at BIGINT NOT NULL,
            transaction_count INTEGER NOT NULL DEFAULT 0,
            UNIQUE(data_source_type, data_source_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_extraction_sources_job ON financial_extraction_sources(extraction_job_id)",
        [],
    )?;

    // Track financial extraction attempts by source system/account
    conn.execute(
        "CREATE TABLE IF NOT EXISTS financial_extraction_attempts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_type VARCHAR NOT NULL,
            source_account_id INTEGER NOT NULL,
            attempted_at BIGINT NOT NULL,
            total_items_scanned INTEGER NOT NULL DEFAULT 0,
            transactions_extracted INTEGER NOT NULL DEFAULT 0,
            status VARCHAR NOT NULL,
            error_message VARCHAR
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_extraction_attempts_source ON financial_extraction_attempts(source_type, source_account_id, attempted_at DESC)",
        [],
    )?;

    // Create financial_patterns table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS financial_patterns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,

            -- Pattern identity
            name VARCHAR NOT NULL,
            regex_pattern VARCHAR NOT NULL,
            description VARCHAR,
            sender_email VARCHAR,
            sender_email VARCHAR,

            -- Pattern metadata
            document_type VARCHAR NOT NULL,
            status VARCHAR NOT NULL,

            -- Capture group indices (which regex group contains each field)
            amount_group INTEGER NOT NULL,
            vendor_group INTEGER,
            source_vendor_group INTEGER,
            destination_vendor_group INTEGER,
            date_group INTEGER,
            reference_group INTEGER,
            currency_group INTEGER,

            -- Management flags
            is_default BOOLEAN DEFAULT false,
            is_active BOOLEAN DEFAULT true,

            -- Usage statistics
            match_count INTEGER DEFAULT 0,
            last_matched_at BIGINT,

            -- Timestamps
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,

            -- Uniqueness constraints
            UNIQUE(name)
        )",
        [],
    )?;

    if !table_has_column(conn, "financial_patterns", "source_vendor_group")? {
        conn.execute(
            "ALTER TABLE financial_patterns ADD COLUMN source_vendor_group INTEGER",
            [],
        )?;
    }

    if !table_has_column(conn, "financial_patterns", "destination_vendor_group")? {
        conn.execute(
            "ALTER TABLE financial_patterns ADD COLUMN destination_vendor_group INTEGER",
            [],
        )?;
    }

    if !table_has_column(conn, "financial_patterns", "reference_group")? {
        conn.execute(
            "ALTER TABLE financial_patterns ADD COLUMN reference_group INTEGER",
            [],
        )?;
    }

    if !table_has_column(conn, "financial_patterns", "currency_group")? {
        conn.execute(
            "ALTER TABLE financial_patterns ADD COLUMN currency_group INTEGER",
            [],
        )?;
    }

    if !table_has_column(conn, "financial_patterns", "sender_email")? {
        conn.execute(
            "ALTER TABLE financial_patterns ADD COLUMN sender_email VARCHAR",
            [],
        )?;
    }

    if table_has_column(conn, "financial_patterns", "confidence")? {
        rebuild_financial_patterns_table(conn)?;
    }

    if table_sql_contains(conn, "financial_patterns", "UNIQUE(regex_pattern)")? {
        rebuild_financial_patterns_table(conn)?;
    }

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_patterns_active ON financial_patterns(is_active)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_patterns_type ON financial_patterns(document_type)",
        [],
    )?;

    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_financial_patterns_regex_sender_unique
         ON financial_patterns(regex_pattern, sender_email)
         WHERE sender_email IS NOT NULL",
        [],
    )?;

    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_financial_patterns_regex_unique_null_sender
         ON financial_patterns(regex_pattern)
         WHERE sender_email IS NULL",
        [],
    )?;

    // Create email_folders table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS email_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            credential_id INTEGER NOT NULL,
            name VARCHAR NOT NULL,
            display_name VARCHAR,
            imap_path VARCHAR NOT NULL,
            folder_type VARCHAR,
            parent_folder_id INTEGER,
            uidvalidity INTEGER,
            last_synced_uid INTEGER,
            oldest_synced_uid INTEGER,
            total_messages INTEGER DEFAULT 0,
            unread_messages INTEGER DEFAULT 0,
            is_subscribed BOOLEAN DEFAULT true,
            is_selectable BOOLEAN DEFAULT true,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            last_synced_at BIGINT,
            UNIQUE(credential_id, imap_path),
            FOREIGN KEY(credential_id) REFERENCES credentials_metadata(id) ON DELETE CASCADE,
            FOREIGN KEY(parent_folder_id) REFERENCES email_folders(id) ON DELETE SET NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_email_folders_credential ON email_folders(credential_id)",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_email_folders_type ON email_folders(credential_id, folder_type)", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_email_folders_parent ON email_folders(parent_folder_id)",
        [],
    )?;

    if !table_has_column(conn, "email_folders", "oldest_synced_uid")? {
        conn.execute(
            "ALTER TABLE email_folders ADD COLUMN oldest_synced_uid INTEGER",
            [],
        )?;
    }

    // Create email_labels table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS email_labels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            credential_id INTEGER NOT NULL,
            name VARCHAR NOT NULL,
            display_name VARCHAR,
            label_type VARCHAR NOT NULL,
            color VARCHAR,
            message_count INTEGER DEFAULT 0,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            UNIQUE(credential_id, name),
            FOREIGN KEY(credential_id) REFERENCES credentials_metadata(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_email_labels_credential ON email_labels(credential_id)",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_email_labels_type ON email_labels(credential_id, label_type)", [])?;

    // Create email_label_associations table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS email_label_associations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email_id INTEGER NOT NULL,
            label_id INTEGER NOT NULL,
            created_at BIGINT NOT NULL,
            UNIQUE(email_id, label_id),
            FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE,
            FOREIGN KEY(label_id) REFERENCES email_labels(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_email_label_assoc_email ON email_label_associations(email_id)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_email_label_assoc_label ON email_label_associations(label_id)", [])?;

    tracing::info!("Database migrations completed successfully");

    Ok(())
}

/// Check if database tables exist
#[allow(dead_code)]
pub fn has_schema(conn: &Connection) -> anyhow::Result<bool> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='agent_sessions'")?;
    Ok(stmt.exists([])?)
}

/// Migrate existing email folders and labels to new normalized schema
#[allow(dead_code)]
pub fn migrate_folders_and_labels(conn: &mut Connection) -> anyhow::Result<()> {
    let tx = conn.transaction()?;

    tracing::info!("Starting email folders and labels migration");

    // Check if migration already ran (folder_id column exists in emails table)
    let has_folder_id: bool = tx.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('emails') WHERE name='folder_id'",
        [],
        |row| {
            let count: i64 = row.get(0)?;
            Ok(count > 0)
        },
    )?;

    if has_folder_id {
        tracing::info!("Migration already completed, ensuring email UID uniqueness");

        // Dedupe any existing rows before creating unique index
        let dup_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM (
                SELECT credential_id, folder_id, uid, COUNT(*) as c
                FROM emails
                WHERE folder_id IS NOT NULL
                GROUP BY credential_id, folder_id, uid
                HAVING c > 1
            )",
            [],
            |row| row.get(0),
        )?;

        if dup_count > 0 {
            tracing::warn!(
                "Found {} duplicate email UID groups, deduplicating",
                dup_count
            );

            tx.execute(
                "CREATE TEMP TABLE email_uid_dedupe AS
                 SELECT e.id AS duplicate_id,
                        (SELECT MIN(e2.id) FROM emails e2
                         WHERE e2.credential_id = e.credential_id
                           AND e2.folder_id = e.folder_id
                           AND e2.uid = e.uid) AS canonical_id
                 FROM emails e
                 WHERE e.id != (
                     SELECT MIN(e2.id) FROM emails e2
                     WHERE e2.credential_id = e.credential_id
                       AND e2.folder_id = e.folder_id
                       AND e2.uid = e.uid
                 )",
                [],
            )?;

            // Preserve label associations without violating uniqueness
            tx.execute(
                "INSERT OR IGNORE INTO email_label_associations (email_id, label_id, created_at)
                 SELECT d.canonical_id, ela.label_id, ela.created_at
                 FROM email_label_associations ela
                 JOIN email_uid_dedupe d ON d.duplicate_id = ela.email_id",
                [],
            )?;
            tx.execute(
                "DELETE FROM email_label_associations
                 WHERE email_id IN (SELECT duplicate_id FROM email_uid_dedupe)",
                [],
            )?;

            // Re-point other tables that reference emails
            tx.execute(
                "UPDATE email_attachments
                 SET email_id = (SELECT canonical_id FROM email_uid_dedupe WHERE duplicate_id = email_attachments.email_id)
                 WHERE email_id IN (SELECT duplicate_id FROM email_uid_dedupe)",
                [],
            )?;
            tx.execute(
                "UPDATE events
                 SET email_id = (SELECT canonical_id FROM email_uid_dedupe WHERE duplicate_id = events.email_id)
                 WHERE email_id IN (SELECT duplicate_id FROM email_uid_dedupe)",
                [],
            )?;
            tx.execute(
                "UPDATE contacts
                 SET email_id = (SELECT canonical_id FROM email_uid_dedupe WHERE duplicate_id = contacts.email_id)
                 WHERE email_id IN (SELECT duplicate_id FROM email_uid_dedupe)",
                [],
            )?;

            tx.execute(
                "DELETE FROM emails WHERE id IN (SELECT duplicate_id FROM email_uid_dedupe)",
                [],
            )?;

            tx.execute("DROP TABLE email_uid_dedupe", [])?;
        }

        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_emails_unique_uid
             ON emails(credential_id, folder_id, uid)",
            [],
        )?;

        tx.commit()?;
        return Ok(());
    }

    // Step 1: Extract unique folders from emails table and insert into email_folders
    tracing::info!("Migrating folders from emails table");
    tx.execute(
        "INSERT INTO email_folders (credential_id, name, imap_path, created_at, updated_at)
         SELECT DISTINCT credential_id, folder, folder, strftime('%s', 'now') * 1000, strftime('%s', 'now') * 1000
         FROM emails
         WHERE folder IS NOT NULL",
        [],
    )?;

    // Step 2: Add folder_id column to emails table
    tracing::info!("Adding folder_id column to emails table");
    tx.execute("ALTER TABLE emails ADD COLUMN folder_id INTEGER", [])?;

    // Step 3: Update emails.folder_id based on folder string
    tracing::info!("Updating emails with folder_id references");
    tx.execute(
        "UPDATE emails
         SET folder_id = (
             SELECT id FROM email_folders
             WHERE email_folders.credential_id = emails.credential_id
               AND email_folders.imap_path = emails.folder
         )",
        [],
    )?;

    // Step 4: Make folder_id NOT NULL (ensure all emails have valid folder_id)
    tracing::info!("Validating folder_id references");
    tx.execute("UPDATE emails SET folder_id = NULL WHERE folder_id = 0", [])?;

    // Step 5: Extract labels from emails table and insert into email_labels
    tracing::info!("Migrating labels from emails table");
    let mut stmt = tx.prepare("SELECT DISTINCT credential_id, labels FROM emails WHERE labels IS NOT NULL AND labels != '[]' AND labels != 'null'")?;
    let rows: Result<Vec<_>, _> = stmt
        .query_map([], |row| {
            let cred_id: i64 = row.get(0)?;
            let labels_json: String = row.get(1)?;
            Ok((cred_id, labels_json))
        })?
        .collect();
    drop(stmt);
    let label_rows: Vec<(i64, String)> = rows?;

    for (cred_id, labels_json) in label_rows {
        if let Ok(label_list) = serde_json::from_str::<Vec<String>>(&labels_json) {
            for label_name in label_list {
                if !label_name.is_empty() {
                    let now = chrono::Utc::now().timestamp_millis();
                    tx.execute(
                        "INSERT OR IGNORE INTO email_labels (credential_id, name, label_type, created_at, updated_at)
                         VALUES (?, ?, 'user', ?, ?)",
                        rusqlite::params![cred_id, label_name.as_str(), now, now],
                    )?;
                }
            }
        }
    }

    // Step 6: Create email_label_associations from existing labels
    tracing::info!("Creating email_label_associations");
    tx.execute(
        "INSERT INTO email_label_associations (email_id, label_id, created_at)
         SELECT e.id, l.id, strftime('%s', 'now') * 1000
         FROM emails e
         CROSS JOIN json_each(e.labels) as label_name
         JOIN email_labels l ON l.credential_id = e.credential_id AND l.name = label_name.value
         WHERE e.labels IS NOT NULL AND e.labels != '[]' AND e.labels != 'null'",
        [],
    )?;

    // Step 7: Update download_items.source_folder -> source_folder_id
    tracing::info!("Migrating download_items.source_folder");
    tx.execute(
        "ALTER TABLE download_items ADD COLUMN source_folder_id INTEGER",
        [],
    )?;
    tx.execute(
        "UPDATE download_items
         SET source_folder_id = (
             SELECT email_folders.id FROM email_folders
             JOIN download_jobs ON download_jobs.id = download_items.job_id
             WHERE email_folders.credential_id = download_jobs.credential_id
               AND email_folders.imap_path = download_items.source_folder
         )
         WHERE source_folder IS NOT NULL",
        [],
    )?;

    // Step 8: Update indexes - drop old index BEFORE dropping the column
    tracing::info!("Updating indexes");
    tx.execute("DROP INDEX IF EXISTS idx_emails_folder_date", [])?;

    // Step 9: Drop old columns
    tracing::info!("Dropping old folder and labels columns");
    tx.execute("ALTER TABLE emails DROP COLUMN folder", [])?;
    tx.execute("ALTER TABLE emails DROP COLUMN labels", [])?;
    tx.execute("ALTER TABLE download_items DROP COLUMN source_folder", [])?;

    // Step 10: Create new index with folder_id
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_emails_folder_date ON emails(folder_id, date_received DESC)",
        [],
    )?;

    // Step 11: Ensure unique UID per credential + folder
    tx.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_emails_unique_uid
         ON emails(credential_id, folder_id, uid)",
        [],
    )?;

    tx.commit()?;

    tracing::info!("Email folders and labels migration completed successfully");

    Ok(())
}
