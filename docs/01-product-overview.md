# Dwata Product Overview (Current Focus)

Dwata is currently focused on one end-to-end workflow:

1. Ingest email data locally via IMAP
2. Build a unified local document store (emails + attachments)
3. Extract structured financial signals from repeated financial emails

The product is local-first. Email content, attachments, search indexes, and extracted records are processed and stored on your machine.

## What Works Today

- IMAP credential and mailbox sync pipeline
- Background download jobs and item-level tracking
- Email + attachment persistence in SQLite
- Unified `documents` model for search and downstream extraction
- Tantivy-based document indexing/search
- Reverse-template financial extraction pipeline
- Typed financial modeling direction (`parse into trusted types`)

## What Is In Focus Right Now

Current engineering focus is financial email extraction quality and reliability, not broad multi-domain assistant features.

Recent implementation direction:

- Regex-first extraction removed in favor of template-driven extraction
- Bill/transaction labeling and template translation added
- Multi-cluster sender matching and translated template preview added
- DB migrations centered on credential -> sync -> email -> attachment -> document lifecycle

## Product Boundary (Current)

In-scope:

- Financial data from repeated sender templates in email
- Deterministic extraction behavior after one-time template interpretation
- Human-guided enrichment roadmap for unresolved party/vendor identity

Out-of-scope (for now):

- Generic all-domain personal assistant behavior
- Large static API/agent catalogs as primary docs
- Per-email LLM extraction as the default runtime path

## Read Next

- `docs/02-current-architecture.md`
- `docs/03-type-driven-financial-extraction.md`
- `docs/04-run-from-source.md`
- `docs/05-blog-financial-email-extraction.md`
