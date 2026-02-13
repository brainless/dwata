# Task: Attachment Text Extraction and Indexing (Deferred)

## Objective

Implement extraction of attachment content into searchable text and index it in Tantivy.

## Current Decision

- Not implemented in current phase.
- Attachments may exist as documents, but extracted text pipeline is deferred.

## Deliverables

- Attachment extraction job framework (no UI-first dependency).
- Typed extraction state and result contract.
- Tantivy updates for extracted attachment text.

## Type Contracts (Strict)

Add shared types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum AttachmentExtractionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AttachmentExtractionResult {
    pub attachment_document_id: i64,
    pub status: AttachmentExtractionStatus,
    pub extracted_text: Option<String>,
    pub extractor_version: String,
    pub extracted_at: i64,
    pub error_message: Option<String>,
}
```

DB additions (migration):
- `documents`:
  - `extraction_status VARCHAR` (enum check)
  - `extraction_error VARCHAR`
  - `extracted_at BIGINT`
  - `extractor_version VARCHAR`
  - `extracted_text_preview VARCHAR` (bounded, optional)
- Optional normalized table:
  - `document_extraction_results(document_id, status, text, extractor_version, created_at)`
  - use this if full text is too large for `documents`.

## Extraction Scope

- Parse supported attachment types:
  - `text/plain`
  - `text/csv`
  - `application/pdf` (via selected parser)
  - office docs (optional phase 2)
- Store extraction output in SQLite metadata table/column (bounded size + checksum versioning).
- Push extracted text into Tantivy `attachment_text` field.
- Track extraction status with strict enum.

## Operational Requirements

- Background job with retry policy and dead-letter handling.
- Size/type limits to prevent runaway CPU/memory use.
- Deterministic re-extraction when parser version changes.

Concrete limits:
- max attachment size for extraction: 25 MB (configurable).
- max extracted text indexed: 256 KB per attachment.
- timeout per file: 30s default.

Retry policy:
- 3 attempts with exponential backoff (1m, 5m, 30m).
- mark `Skipped` for unsupported type or size limit.
- mark `Failed` only for actionable transient/permanent parser errors.

## Pipeline Design

1. Select candidate attachments with `Pending` status.
2. Resolve attachment file path from storage metadata.
3. Extract text using typed extractor based on MIME.
4. Persist extraction result atomically in SQLite.
5. Upsert Tantivy document with `attachment_text`.
6. Emit metrics/log event.

## Re-Extraction Rules

Trigger re-extraction when any changes:
- attachment checksum changes
- extractor version changes
- extraction settings change (limits/parser toggles)

Use deterministic key:
- `extraction_fingerprint = sha256(checksum + extractor_version + settings_hash)`

## Testing Matrix

Unit tests:
- MIME routing and fallback behavior.
- size/time limit enforcement.
- extraction status transitions.

Integration tests:
- successful extraction updates DB + Tantivy.
- unsupported MIME marked `Skipped`.
- parser failure marked `Failed` with error message.
- retry job resumes incomplete queue.

## Acceptance Criteria

- Attachment search returns relevant hits with snippets.
- Failed extraction does not block email ingestion.
- Reindexing can rebuild from SQLite + attachment file store.
