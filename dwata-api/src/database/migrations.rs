use rusqlite::Connection;

/// Run all database migrations
#[allow(dead_code)]
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
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

    // Create financial_transactions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS financial_transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,

            -- Source tracking (agnostic to source type)
            source_type VARCHAR NOT NULL,
            source_id VARCHAR NOT NULL,
            extraction_job_id INTEGER,

            -- Transaction data
            document_type VARCHAR NOT NULL,
            description VARCHAR NOT NULL,
            amount DOUBLE NOT NULL,
            currency VARCHAR NOT NULL DEFAULT 'USD',
            transaction_date VARCHAR NOT NULL,

            -- Additional fields
            category VARCHAR,
            vendor VARCHAR,
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
            UNIQUE(source_type, source_id, amount, vendor, transaction_date, document_type)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_transactions_source ON financial_transactions(source_type, source_id)",
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

    // Track which sources have been processed for financial extraction
    conn.execute(
        "CREATE TABLE IF NOT EXISTS financial_extraction_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_type VARCHAR NOT NULL,
            source_id VARCHAR NOT NULL,
            extraction_job_id INTEGER,
            extracted_at BIGINT NOT NULL,
            transaction_count INTEGER NOT NULL DEFAULT 0,
            UNIQUE(source_type, source_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_extraction_sources_job ON financial_extraction_sources(extraction_job_id)",
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

            -- Pattern metadata
            document_type VARCHAR NOT NULL,
            status VARCHAR NOT NULL,
            confidence FLOAT NOT NULL,

            -- Capture group indices (which regex group contains each field)
            amount_group INTEGER NOT NULL,
            vendor_group INTEGER,
            date_group INTEGER,

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
            UNIQUE(name),
            UNIQUE(regex_pattern)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_patterns_active ON financial_patterns(is_active)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_financial_patterns_type ON financial_patterns(document_type)",
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

    // Seed default financial patterns
    conn.execute(
        "INSERT INTO financial_patterns
            (name, regex_pattern, description, document_type, status, confidence,
             amount_group, vendor_group, date_group, is_default, is_active,
             match_count, created_at, updated_at)
        VALUES
            -- Payment Confirmation Patterns (5)
            ('payment_to_vendor',
             '(?i)payment of \\$?([\\d,]+\\.?\\d{0,2}) to ([A-Za-z\\s]+)',
             'Matches: \"payment of $150.00 to Comcast\"',
             'payment-confirmation', 'paid', 0.90,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            ('paid_amount_to_vendor',
             '(?i)paid \\$?([\\d,]+\\.?\\d{0,2}) to ([A-Za-z\\s]+)',
             'Matches: \"paid $99 to Adobe\"',
             'payment-confirmation', 'paid', 0.88,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            ('your_payment_to_vendor',
             '(?i)your \\$?([\\d,]+\\.?\\d{0,2}) payment to ([A-Za-z\\s]+)',
             'Matches: \"Your $50.00 payment to Netflix\"',
             'payment-confirmation', 'paid', 0.87,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            ('successfully_paid_to_vendor',
             '(?i)successfully paid \\$?([\\d,]+\\.?\\d{0,2}) to ([A-Za-z\\s]+)',
             'Matches: \"successfully paid $1,200.00 to Chase\"',
             'payment-confirmation', 'paid', 0.92,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            ('payment_processed_to_vendor',
             '(?i)payment processed:? \\$?([\\d,]+\\.?\\d{0,2}) to ([A-Za-z\\s]+)',
             'Matches: \"payment processed: $45.99 to Spotify\"',
             'payment-confirmation', 'paid', 0.91,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            -- Bill/Invoice Due Patterns (4)
            ('bill_due_explicit',
             '(?i)bill (?:of|for) \\$?([\\d,]+\\.?\\d{0,2}) (?:is )?due (?:on )?([A-Za-z]+ \\d{1,2})',
             'Matches: \"bill of $99.99 is due on Feb 10\"',
             'bill', 'pending', 0.88,
             1, NULL, 2,
             true, true,
             0, 0, 0),

            ('invoice_due_date',
             '(?i)invoice for \\$?([\\d,]+\\.?\\d{0,2}) due ([A-Za-z]+ \\d{1,2})',
             'Matches: \"invoice for $3,500 due January 25\"',
             'invoice', 'pending', 0.89,
             1, NULL, 2,
             true, true,
             0, 0, 0),

            ('vendor_bill_due',
             '(?i)(?:your )?([A-Za-z]+) bill (?:\\(?\\$?([\\d,]+\\.?\\d{0,2})\\)?) is due ([A-Za-z]+ \\d{1,2})',
             'Matches: \"Your Adobe bill ($99) is due Feb 10\"',
             'bill', 'pending', 0.87,
             2, 1, 3,
             true, true,
             0, 0, 0),

            ('due_amount_by_date',
             '(?i)due:? \\$?([\\d,]+\\.?\\d{0,2}) by (\\d{2}/\\d{2}/\\d{4})',
             'Matches: \"due: $150.00 by 02/05/2026\"',
             'bill', 'pending', 0.86,
             1, NULL, 2,
             true, true,
             0, 0, 0),

            -- Payment Received Patterns (3)
            ('received_payment_from',
             '(?i)received (?:a payment of )?\\$?([\\d,]+\\.?\\d{0,2}) from ([A-Za-z\\s]+)',
             'Matches: \"received $3,500.00 from Acme Corp\"',
             'invoice', 'paid', 0.92,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            ('payment_of_from',
             '(?i)payment of \\$?([\\d,]+\\.?\\d{0,2}) from ([A-Za-z\\s]+)',
             'Matches: \"payment of $2,000 from TechStart Inc\"',
             'invoice', 'paid', 0.90,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            ('you_received_payment',
             '(?i)you received (?:a payment:? )?\\$?([\\d,]+\\.?\\d{0,2}) from ([A-Za-z\\s]+)',
             'Matches: \"You received a payment: $1,500 from Client Name\"',
             'invoice', 'paid', 0.91,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            -- Overdue/Late Patterns (3)
            ('payment_overdue',
             '(?i)payment of \\$?([\\d,]+\\.?\\d{0,2}) (?:is |to ([A-Za-z\\s]+) )?(?:is )?overdue',
             'Matches: \"payment of $1,200 is overdue\" or \"payment of $1,200 to Chase is overdue\"',
             'bill', 'overdue', 0.93,
             1, 2, NULL,
             true, true,
             0, 0, 0),

            ('amount_past_due',
             '(?i)\\$?([\\d,]+\\.?\\d{0,2}) payment past due',
             'Matches: \"$450.00 payment past due\"',
             'bill', 'overdue', 0.91,
             1, NULL, NULL,
             true, true,
             0, 0, 0),

            ('overdue_bill_days',
             '(?i)overdue bill:? \\$?([\\d,]+\\.?\\d{0,2})(?: \\((\\d+) days? late\\))?',
             'Matches: \"overdue bill: $99 (3 days late)\"',
             'bill', 'overdue', 0.90,
             1, NULL, NULL,
             true, true,
             0, 0, 0)
        ON CONFLICT (regex_pattern) DO NOTHING",
        [],
    )?;

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
        tracing::info!("Migration already completed, skipping");
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

    tx.commit()?;

    tracing::info!("Email folders and labels migration completed successfully");

    Ok(())
}
