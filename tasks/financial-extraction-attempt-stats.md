# Task: Financial Extraction Attempt Stats

## Objective
Add account-level tracking for each financial extraction attempt, keyed by data source system (imap, google-drive, dropbox, onedrive, local-file) and a stable account ID (credential id).

## Scope
- Extend `SourceType` with `LocalFile`.
- Add `financial_extraction_attempts` table to store per-attempt stats (account + source system).
- Record attempt stats from the financial extraction manager.

## Notes
- Use `credentials_metadata.id` as `source_account_id` for all sources (including local files).
- Use `SourceType` values for `source_type`.
- Record attempts even when no transactions are found; mark failures when extraction cannot run.
