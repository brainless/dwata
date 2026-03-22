CREATE TABLE email_labels (
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
);

CREATE INDEX idx_email_labels_credential ON email_labels(credential_id);
CREATE INDEX idx_email_labels_type ON email_labels(credential_id, label_type);
