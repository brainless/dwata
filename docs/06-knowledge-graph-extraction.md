# Knowledge Graph Extraction

## Goal

Extract structured entities from emails into a personal Knowledge Graph (KG). Each pass grows the KG incrementally — new emails either create new nodes or link to existing ones via BM25 pre-population.

Accuracy must be practical on small models (< 7B params), which rules out wide one-shot extraction schemas.

---

## Pass Architecture

Four sequential passes, each a narrow extraction task. A document labeler gates which pass executes.

| Pass | Entity types | Gates |
|---|---|---|
| Identity Resolution | `location`, `organisation`, `person` | Always |
| Financial Extraction | `bill`, `transaction`, `subscription` | Financial signals |
| Event Extraction | `event` | Meeting/appointment signals |
| Order Extraction | `order` | Order/shipping signals |

After each pass the server persists entities before the next pass starts.

### Entity Types

Defined as `NamedEntityKind` enum in `dwata-agents/src/entity_search.rs`:
```
location · organisation · person · bill · transaction · subscription · order · event
```

### ID Convention

- **Positive integers** — new entity being created this session (LLM assigns)
- **Negative integers** — existing KG node (pre-populated from DB search)

### Pre-population

Before each pass, the server runs BM25 search against entity summaries in the KG. Top matches are injected into the system prompt:

```
## Existing Entities (Pre-populated)
- [Netflix Inc](organisation) — id:-42
  streaming service, monthly billing, billing@netflix.com
- [John Smith](person) — id:-31
  engineer at Acme Corp
```

The LLM reuses existing entities (via negative ID) or creates new ones (positive ID). No search tool — small models are unreliable at deciding when and how to query.

---

## Search Infrastructure

### Entity Search Index

Separate Tantivy index (`dwata-api/src/search/entity_index.rs`) for entity summaries. Fields: `entity_type`, `entity_id`, `name`, `search_summary`.

```rust
// dwata-agents/src/entity_search.rs
pub trait EntitySearchProvider: Send + Sync {
    async fn search_entities(&self, params: &SearchEntitiesParams)
        -> anyhow::Result<Vec<EntitySearchResult>>;
}
```

### Entity Type Manifest

`dwata-agents/src/entity_type_manifest.rs` generates schema docs at runtime:

```rust
pub fn generate_entity_manifest(for_kinds: Option<&[NamedEntityKind]>) -> String
pub fn existing_entities_section(results: &[EntitySearchResult]) -> String
```

No hardcoded entity schemas in prompts — introspection only.

### Pass Context

`dwata-agents/src/kg_pass_context.rs`:

```rust
pub enum KgPassType {
    IdentityResolution,
    FinancialExtraction,
    EventExtraction,
    OrderExtraction,
}

pub struct KgExtractionPass {
    pub pass_type: KgPassType,
    pub existing_entities: Vec<EntitySearchResult>,
    pub source_content: String,
}
```

Usage:

```rust
let pass = KgExtractionPass::new(KgPassType::IdentityResolution, email_content)
    .populate_existing_entities(search_provider.as_deref())
    .await;

let prompt = pass.build_system_prompt();
```

---

## Database

KG tables in `dwata-api/migrations/V6__create_kg_tables.sql`:
```
locations · organisations · organisation_roles · persons · contact_links
subscriptions · orders · events
```

All entity tables include a `search_summary TEXT` column for BM25 indexing.

The index is rebuilt via `dwata-api/src/search/entity_index.rs`:
```rust
pub async fn reindex_all_entities(pool, index) -> Result<()>
```

---

## Extracted Entity → DB Persistence

TODO: `insert_named_entity()` in `dwata-api/src/database/` — single function accepting `NamedEntityKind` variant and typed payload, returns persistent KG ID. Called after each pass to persist and get stable IDs for downstream FK resolution.

---

## Open Questions

1. **Pre-population list cap** — how many candidates before context pressure degrades accuracy. Start: 5 per entity type.

2. **Entity resolution quality** — over-merge vs under-merge rate needs evaluation data.

3. **User-in-the-loop** — entity deduplication (especially orgs/persons) may need a review queue.
