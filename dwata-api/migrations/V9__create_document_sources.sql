CREATE TABLE document_sources (
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
    updated_at BIGINT NOT NULL,
    FOREIGN KEY(credential_id) REFERENCES credentials_metadata(id) ON DELETE SET NULL
);

CREATE INDEX idx_document_sources_type ON document_sources(source_type);
CREATE INDEX idx_document_sources_credential ON document_sources(credential_id);
