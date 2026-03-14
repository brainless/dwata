-- Drop old download job tracking tables (replaced by in-memory state in EmailSyncManager)
DROP TABLE IF EXISTS download_items;
DROP TABLE IF EXISTS download_jobs;

-- Per-credential email sync settings (only persisted state needed)
CREATE TABLE IF NOT EXISTS email_sync_settings (
    credential_id INTEGER PRIMARY KEY,
    is_paused     INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);
