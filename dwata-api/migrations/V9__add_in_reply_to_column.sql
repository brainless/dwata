-- Add in_reply_to column to track email reply relationships
ALTER TABLE emails ADD COLUMN in_reply_to VARCHAR;

-- Index for efficient reply lookup
CREATE INDEX idx_emails_in_reply_to ON emails(in_reply_to);

-- Index for message_id lookups (needed for reply correlation)
CREATE INDEX idx_emails_message_id_lookup ON emails(message_id) WHERE message_id IS NOT NULL;
