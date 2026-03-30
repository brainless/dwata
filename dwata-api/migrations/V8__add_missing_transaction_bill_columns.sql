-- V8: Add missing columns to transactions and bills tables
-- Aligns DB schema with Rust Transaction and Bill types

-- Add missing columns to transactions table
ALTER TABLE transactions ADD COLUMN data_source_type TEXT;
ALTER TABLE transactions ADD COLUMN data_source_id TEXT;
ALTER TABLE transactions ADD COLUMN status TEXT; -- paid, cancelled, refunded
ALTER TABLE transactions ADD COLUMN source_file TEXT;
ALTER TABLE transactions ADD COLUMN extracted_at INTEGER; -- UTC ms

-- Add missing columns to bills table
-- Note: subscription_id was already added in V7
ALTER TABLE bills ADD COLUMN data_source_type TEXT;
ALTER TABLE bills ADD COLUMN data_source_id TEXT;
ALTER TABLE bills ADD COLUMN status TEXT; -- received, unpaid, paid, overdue, cancelled
ALTER TABLE bills ADD COLUMN category TEXT; -- snake_case TransactionCategory variant
