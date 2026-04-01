-- Drop the unreliable is_answered column
-- We now compute this dynamically using the Sent folder and in_reply_to relationships
ALTER TABLE emails DROP COLUMN is_answered;
