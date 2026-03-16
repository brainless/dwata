CREATE TABLE email_folders (
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
);

CREATE INDEX idx_email_folders_credential ON email_folders(credential_id);
CREATE INDEX idx_email_folders_type ON email_folders(credential_id, folder_type);
CREATE INDEX idx_email_folders_parent ON email_folders(parent_folder_id);
