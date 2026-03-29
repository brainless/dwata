-- V7: Add subscription_id FK to bills
--
-- V6 was originally missing this column. This migration adds it for databases
-- that already ran V6. Fresh databases get the column from V6 directly.
ALTER TABLE bills ADD COLUMN subscription_id INTEGER REFERENCES subscriptions(id);
