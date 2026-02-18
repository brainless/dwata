CREATE TABLE download_jobs (
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
    last_sync_at BIGINT,
    FOREIGN KEY (credential_id) REFERENCES credentials_metadata(id) ON DELETE CASCADE
);

CREATE INDEX idx_download_jobs_status ON download_jobs(status, updated_at);
CREATE INDEX idx_download_jobs_credential ON download_jobs(credential_id);
