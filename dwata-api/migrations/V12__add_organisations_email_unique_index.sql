-- V12: Add unique index on organisations.email
-- Prevents duplicate organisation rows for the same email address.
-- Matches the existing pattern on persons.email (idx_persons_email).
CREATE UNIQUE INDEX IF NOT EXISTS idx_organisations_email
    ON organisations(email) WHERE email IS NOT NULL;
