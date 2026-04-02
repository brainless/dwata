-- Indexes to speed sender/thread aggregate ranking passes.
CREATE INDEX IF NOT EXISTS idx_emails_credential_from_date
    ON emails(credential_id, from_address, date_received DESC);

CREATE INDEX IF NOT EXISTS idx_emails_in_reply_to
    ON emails(in_reply_to);

CREATE INDEX IF NOT EXISTS idx_emails_credential_thread
    ON emails(credential_id, thread_id);
