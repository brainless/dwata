# Current Architecture and Data Flow

This document describes the architecture that matches the current branch and migrations.

## Runtime Components

- `dwata-api`: Rust backend (Actix), SQLite persistence, sync jobs, extraction APIs
- `dwata-agents`: template-based financial extraction logic and CLI tooling
- `nocodo-llm-sdk` (Cargo alias to sibling `llm-sdk` repo): provider abstraction for one-time template interpretation calls

## Primary Data Flow

1. Credentials are stored in `credentials_metadata`.
2. Sync jobs are tracked in `download_jobs` and `download_items`.
3. Mailbox/folder taxonomy is persisted in `email_folders`, `email_labels`, and `email_label_associations`.
4. Raw emails are stored in `emails`; attachments in `email_attachments`.
5. Source abstraction is captured in `document_sources`.
6. Unified searchable records are persisted in `documents`.
7. Financial extraction operates over sender-specific repeated document patterns.

## What Migrations Say About Product Focus

The migration sequence (`V1` -> `V10`) clearly emphasizes:

- Reliable IMAP ingestion and resumable background downloads
- Normalized email and attachment storage
- A unified document layer for search/indexing and downstream extraction

Notably, there are no migrations for broad task/project/life-goal orchestration in the current active path; the schema investment is in ingestion, documents, and extraction-ready structure.

## Extraction Strategy Placement

Extraction is layered on top of the document/email foundation:

- Group similar sender emails into clusters
- Derive a reverse template from repeated structure
- Use LLM calls once per sender/template to classify and map placeholders
- Run deterministic extraction for individual emails without per-email LLM calls

For the design contract, see `docs/03-type-driven-financial-extraction.md`.

## Operational Notes

- Dev runbook: `docs/04-run-from-source.md`
- The API defaults to `127.0.0.1:8080`; configure a different host and port
  when your OAuth client requires a specific callback URI.
