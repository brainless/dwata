# Task: Unified Documents API and Keyset Pagination

## Objective

Implement a unified read API for documents (emails, attachments, files) backed by the new `documents` and `document_sources` tables.

## Why

- Current email list endpoint uses offset paging and does not provide a true total count in all cases.
- We need one typed browse surface for `email`, `attachment`, and `file`.

## Non-Goals

- Do not remove `/api/emails` in this task.
- Do not implement Tantivy search in this task.

## API Contracts (Strict Types)

Add new shared types in `shared-types/src/document.rs` (or split file if preferred):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DocumentSortBy {
    ReceivedAtDesc,
    ModifiedAtDesc,
    CreatedAtDesc,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DocumentCursor {
    pub sort_value: i64,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListDocumentsRequest {
    pub source_id: Option<i64>,
    pub credential_id: Option<i64>,
    pub kind: Option<DocumentKind>,
    pub parent_document_id: Option<i64>,
    pub limit: Option<usize>, // default 50, max 200
    pub cursor: Option<DocumentCursor>,
    pub sort_by: Option<DocumentSortBy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListDocumentsResponse {
    pub documents: Vec<Document>,
    pub next_cursor: Option<DocumentCursor>,
    pub has_more: bool,
}
```

Rules:
- `offset` is not allowed in this endpoint.
- `sort_by` default is `ReceivedAtDesc`.
- `limit > 200` returns `400`.

## Endpoint Definitions

- `GET /api/documents`
  - Query params map to `ListDocumentsRequest`.
  - Returns `ListDocumentsResponse`.
- `GET /api/documents/{id}`
  - Returns one `Document` or `404`.

## Query Semantics

Keyset pagination query shape (for `ReceivedAtDesc`):

```sql
SELECT d.*
FROM documents d
JOIN document_sources s ON s.id = d.source_id
WHERE
  (?1 IS NULL OR d.source_id = ?1)
  AND (?2 IS NULL OR s.credential_id = ?2)
  AND (?3 IS NULL OR d.kind = ?3)
  AND (?4 IS NULL OR d.parent_document_id = ?4)
  AND (
    ?5 IS NULL OR
    d.date_received < ?5 OR
    (d.date_received = ?5 AND d.id < ?6)
  )
ORDER BY d.date_received DESC, d.id DESC
LIMIT ?7
```

For each sort mode:
- `ReceivedAtDesc` => `(date_received, id)`
- `ModifiedAtDesc` => `(date_modified, id)`
- `CreatedAtDesc` => `(created_at, id)`

Null handling:
- Use `COALESCE` fallback chain:
  - Received: `COALESCE(date_received, date_modified, date_created, created_at, 0)`
  - Modified: `COALESCE(date_modified, date_created, created_at, 0)`
  - Created: `COALESCE(date_created, created_at, 0)`

## DB Work Items

- Add new DB module: `dwata-api/src/database/documents.rs`
  - `list_documents(...)`
  - `get_document(...)`
  - `upsert_document_source(...)`
  - `upsert_document(...)`
- Ensure indexes exist for keyset and filters:
  - `(source_id, kind)`
  - `(date_received DESC, id DESC)`
  - `(date_modified DESC, id DESC)`
  - `(created_at DESC, id DESC)`
  - `(parent_document_id)`

## Handler Work Items

- Add `dwata-api/src/handlers/documents.rs`
- Register routes in `dwata-api/src/handlers/mod.rs` and `dwata-api/src/main.rs`.

## Rollout Plan

1. Add endpoints and types.
2. Add GUI data client for `/api/documents` behind feature flag.
3. Migrate reader UI path-by-path.
4. Keep `/api/emails` until full parity is confirmed.

## Testing Matrix

Unit tests:
- Cursor boundary behavior (`<`, `=` and `id` tie-break).
- Null sort-value fallback correctness.
- Filter composition correctness.

Integration tests:
- Seed mixed docs (email/attachment/file) and verify:
  - stable ordering
  - no duplicate rows across pages
  - no missing rows across pages
  - `has_more` and `next_cursor` correctness

API tests:
- `limit=0`, `limit>200`, invalid enum values, malformed cursor.
- `404` for missing document id.

## Acceptance Criteria

- Introduce strict request/response types in `shared-types`:
  - `ListDocumentsRequest`
  - `ListDocumentsResponse`
  - `DocumentCursor`
- Do not use free-form status strings in API contracts.
- Reader page can list mixed document kinds through one endpoint.
- Pagination is stable under inserts.
- Email-specific API (`/api/emails`) continues to work during migration.
