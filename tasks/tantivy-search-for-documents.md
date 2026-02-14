# Task: Tantivy Search for Unified Documents

## Objective

Replace SQLite FTS usage with Tantivy-based search for emails, attachments, and files while keeping SQLite as source-of-record metadata.

## Current Status (2026-02-14)

- TODO: Event-driven upsert/delete triggers into Tantivy are not implemented yet (startup backfill exists).
- TODO: Dedicated resumable `BackfillDocumentsIndexJob` with persisted cursor/commit cadence metadata is not implemented yet.
- Done: GUI search now uses `/api/documents/search`.
- Done: Legacy SQLite FTS search path for email listing has been removed.

## Non-Goals

- Do not implement attachment binary parsing in this task.
- Do not remove legacy SQLite FTS code until parity is verified.

## Tantivy Schema

Required fields:

- `document_id: u64` (indexed + stored, unique key)
- `kind: text` (indexed, fast field via u64 mapping optional)
- `source_id: u64` (indexed, fast field)
- `title: text`
- `from_address: text`
- `body_text: text`
- `attachment_text: text` (can be empty in phase 1)
- `date_received: i64` (fast field)
- `date_modified: i64` (fast field)
- `indexed_at: i64` (fast field)

Tokenizer/analyzer:
- Default tokenizer: `en_stem` for free text fields.
- Exact tokenizer for email/keyword fields (`from_address`, `kind`).
- No fuzzy default in v1.

## API Contracts (Strict Types)

Add shared types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SearchField {
    Any,
    Title,
    FromAddress,
    BodyText,
    AttachmentText,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchTerm {
    pub field: SearchField,
    pub value: String,
    pub is_phrase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchDocumentsRequest {
    pub terms: Vec<SearchTerm>,
    pub kind: Option<DocumentKind>,
    pub source_id: Option<i64>,
    pub limit: Option<usize>, // default 25, max 100
    pub offset: Option<usize>, // allowed for ranked search pages
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchHit {
    pub document_id: i64,
    pub score: f32,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchDocumentsResponse {
    pub hits: Vec<SearchHit>,
    pub documents: Vec<Document>,
    pub total_hits: usize,
}
```

Rules:
- Empty `terms` returns `400`.
- `limit > 100` returns `400`.
- `offset` is allowed for search pagination only.

## Type-Driven Requirements

- Query DSL should be typed (field enum + term structs), not ad-hoc string parsing.

## Backend Work Items

- Add module `dwata-api/src/search/tantivy.rs` with:
  - `build_schema()`
  - `open_or_create_index(path)`
  - `index_document(doc: &Document, extracted: &IndexedTextFields)`
  - `delete_document(document_id)`
  - `search(request: &SearchDocumentsRequest) -> TantivySearchResult`

- Add persistent index path in config:
  - `search.index_path` (default under data dir: `dwata/tantivy-index`).

- Add endpoint:
  - `GET /api/documents/search`
  - handler converts query params into typed request.
  - search returns IDs, then hydrate `documents` from SQLite in one query.

## Indexing Pipeline

Triggers/events to index:
- new/updated email row -> upsert corresponding document index record
- new/updated document row -> upsert index
- deleted document row -> delete from index

Add job:
- `BackfillDocumentsIndexJob`
  - scans `documents` in pages
  - builds index entries
  - commits every N docs
  - stores progress cursor for resume

## Migration & Cutover

1. Keep existing `list_emails_fts` path as fallback.
2. Add Tantivy search endpoint and compare top-N against SQLite FTS for seeded datasets.
3. Switch GUI search requests to `/api/documents/search`.
4. After validation window, disable SQLite FTS route in handlers.
5. Keep FTS table for one release in case rollback is needed, then drop in dedicated migration.

## Testing Matrix

Unit tests:
- Query builder from `SearchDocumentsRequest`.
- Field mapping (`Any`, `Title`, etc.).
- Filter behavior (`kind`, `source_id`).

Integration tests:
- Backfill job indexes all seeded docs.
- Update/delete events reflect in index.
- Search+hydrate returns consistent `documents`.

Relevance tests:
- Phrase match vs term match behavior.
- Sender-specific queries hit `from_address`.
- Snippet generation not empty for body matches.

## Acceptance Criteria

- No production path depends on SQLite FTS for search.
- Search supports filters by `kind` and `source_id`.
- Ranking/highlighting available in API response.
