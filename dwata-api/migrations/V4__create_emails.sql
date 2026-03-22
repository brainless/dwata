CREATE TABLE emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    credential_id INTEGER NOT NULL,
    folder_id INTEGER,
    uid INTEGER NOT NULL,
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
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY(credential_id) REFERENCES credentials_metadata(id),
    FOREIGN KEY(folder_id) REFERENCES email_folders(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX idx_emails_unique_uid ON emails(credential_id, folder_id, uid);
CREATE INDEX idx_emails_credential ON emails(credential_id);
CREATE INDEX idx_emails_credential_date ON emails(credential_id, date_received DESC);
CREATE INDEX idx_emails_folder_date ON emails(folder_id, date_received DESC);
CREATE INDEX idx_emails_message_id ON emails(message_id);
CREATE INDEX idx_emails_from ON emails(from_address);
CREATE INDEX idx_emails_date_sent ON emails(date_sent DESC);
