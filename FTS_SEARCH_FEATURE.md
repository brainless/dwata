# Legacy SQLite FTS Note

## Status
This document is kept for historical context only.

- SQLite FTS5-based email search is no longer used by runtime code.
- Search now uses Tantivy via `GET /api/documents/search`.
- Financial email scan prefiltering also uses Tantivy.
- Existing databases may still contain old `emails_fts` tables; they are unused.

## Current Search Path
- Backend search index module: `dwata-api/src/search/tantivy.rs`
- Search handler: `dwata-api/src/handlers/documents.rs`
- Financial scan handler: `dwata-api/src/handlers/financial.rs`

## Migration/Cleanup Policy
- Code-level FTS creation/maintenance has been removed.
- No destructive migration is applied yet to drop legacy FTS tables from existing user databases.
