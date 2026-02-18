CREATE TABLE download_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER NOT NULL,
    source_identifier VARCHAR NOT NULL,
    source_folder_id INTEGER,
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
    downloaded_at BIGINT,
    FOREIGN KEY(job_id) REFERENCES download_jobs(id) ON DELETE CASCADE,
    FOREIGN KEY(source_folder_id) REFERENCES email_folders(id) ON DELETE SET NULL
);

CREATE INDEX idx_download_items_job_status ON download_items(job_id, status);
CREATE INDEX idx_download_items_source_identifier ON download_items(job_id, source_identifier);
