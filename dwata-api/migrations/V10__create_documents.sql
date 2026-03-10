CREATE TABLE documents (
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
    FOREIGN KEY(source_id) REFERENCES document_sources(id) ON DELETE CASCADE,
    FOREIGN KEY(parent_document_id) REFERENCES documents(id) ON DELETE SET NULL,
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE SET NULL,
    FOREIGN KEY(attachment_id) REFERENCES email_attachments(id) ON DELETE SET NULL
);

CREATE INDEX idx_documents_source_kind ON documents(source_id, kind);
CREATE INDEX idx_documents_parent ON documents(parent_document_id);
CREATE INDEX idx_documents_email_id ON documents(email_id);
CREATE INDEX idx_documents_attachment_id ON documents(attachment_id);
CREATE INDEX idx_documents_received_date ON documents(date_received DESC, id DESC);
CREATE INDEX idx_documents_modified_date ON documents(date_modified DESC, id DESC);
CREATE INDEX idx_documents_created_date ON documents(created_at DESC, id DESC);
CREATE UNIQUE INDEX idx_documents_source_external_uri_unique ON documents(source_id, external_uri) WHERE external_uri IS NOT NULL;
CREATE UNIQUE INDEX idx_documents_source_storage_path_unique ON documents(source_id, storage_path) WHERE storage_path IS NOT NULL;
