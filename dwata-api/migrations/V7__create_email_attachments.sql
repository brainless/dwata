CREATE TABLE email_attachments (
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
    updated_at BIGINT NOT NULL,
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE
);

CREATE INDEX idx_attachments_email ON email_attachments(email_id);
CREATE INDEX idx_attachments_checksum ON email_attachments(checksum);
