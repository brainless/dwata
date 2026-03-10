CREATE TABLE credentials_metadata (
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
);

CREATE INDEX idx_credentials_type_active ON credentials_metadata(credential_type, is_active);
