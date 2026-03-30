# Knowledge Graph Extraction

## Goal

Extract structured entities from emails into a personal Knowledge Graph (KG). Each pass grows the KG incrementally — new emails either create new nodes or link to existing ones via BM25 pre-population.

Accuracy must be practical on small models (< 7B params), which rules out wide one-shot extraction schemas.

---

## Pass Architecture

Four sequential passes, each a narrow extraction task. A document labeler gates which passes execute.

| Pass | Entity types | Gate condition |
|---|---|---|
| Identity Resolution | `location`, `organisation`, `person` | Always |
| Financial Extraction | `bill`, `transaction`, `subscription` | `has_bill \|\| has_transaction` |
| Event Extraction | `event` | `has_event` |
| Order Extraction | `order` | `has_order` |

After each pass the server persists entities before the next pass starts. Entities written in pass N are immediately searchable in pass N+1 via the Tantivy entity index.

If no document label is available (labeler failed or `--all-passes` flag), all four passes run.

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

## Components

### Document Labeler (`dwata-agents/src/kg_email_extractor/document_labeler.rs`)

`TemplateDocumentLabelerAgent` classifies an email to determine which passes to gate in:

```rust
pub struct LabelDocumentParams {
    pub doc_type: DocumentType,    // bill / invoice / receipt / …
    pub has_bill: bool,            // → FinancialExtraction pass
    pub has_transaction: bool,     // → FinancialExtraction pass
    pub has_event: bool,           // → EventExtraction pass
    pub has_order: bool,           // → OrderExtraction pass
}
```

### Entity Search Infrastructure (`dwata-agents/src/entity_search.rs`)

```rust
pub trait EntitySearchProvider: Send + Sync {
    async fn search_entities(&self, params: &SearchEntitiesParams)
        -> anyhow::Result<Vec<EntitySearchResult>>;
}
```

Concrete implementation: `DbEntitySearchProvider` in `dwata-api/src/search/entity_index.rs` (Tantivy-backed).

### Entity Type Manifest (`dwata-agents/src/entity_type_manifest.rs`)

Generates schema docs at runtime — no hardcoded entity schemas in prompts:

```rust
pub fn generate_entity_manifest(for_kinds: Option<&[NamedEntityKind]>) -> String
pub fn existing_entities_section(results: &[EntitySearchResult]) -> String
```

### Pass Context (`dwata-agents/src/kg_pass_context.rs`)

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

### Persistence Trait (`dwata-agents/src/kg_persistence.rs`)

Allows `KgEmailExtractionAgent` to persist entities between passes without depending on `dwata-api`:

```rust
#[async_trait]
pub trait KgPersistenceProvider: Send + Sync {
    async fn persist_pass_result(
        &self,
        params: &ExtractedEntitiesParams,
        source_email_id: Option<i64>,
    ) -> anyhow::Result<()>;
}
```

Concrete implementation: `KgPersistenceLayer` in `dwata-api/src/database/kg_entities.rs`, which also indexes each persisted entity into the Tantivy entity index so the next pass can find it via pre-population.

### KG Extraction Agent (`dwata-agents/src/kg_email_extractor/`)

`KgEmailExtractionAgent` orchestrates the four passes:

```rust
let agent = KgEmailExtractionAgent::new(
    llm_client,
    storage,
    persistence,        // Arc<dyn KgPersistenceProvider>
    model,
    email_content,
)
.with_search_provider(search_provider)  // Arc<dyn EntitySearchProvider>
.with_label(label)                      // LabelDocumentParams for gating
.with_source_email_id(email_id);        // provenance FK

agent.execute(session_id).await?;
```

Within each pass, the agent:
1. Builds the pass-specific system prompt (via `KgExtractionPass`)
2. Drives a `submit_entities` / `confirm_entities` tool-use loop
3. Calls `persist_pass_result` after confirmation
4. Signals the LLM that the next pass is starting

---

## Database

KG tables in `dwata-api/migrations/`:

| Migration | Tables |
|---|---|
| `V6__create_kg_tables.sql` | `locations`, `organisations`, `organisation_roles`, `persons`, `contact_links`, `subscriptions`, `bills`, `transactions`, `orders`, `events` |
| `V7__bills_add_subscription_id.sql` | adds `subscription_id FK` to `bills` |

All entity tables include a `search_summary TEXT` column for BM25 indexing.

The entity index is rebuilt via `reindex_all_entities()` in `dwata-api/src/search/entity_index.rs`.

---

## Running the KG Extraction Pipeline

```bash
# Single email, with document labeling (gated passes)
cargo run --bin extract_kg_entities -- <email_id>

# Skip labeler, run all four passes
cargo run --bin extract_kg_entities -- --all-passes <email_id>
```

The binary:
1. Opens the DB and entity Tantivy index
2. Reindexes all existing KG entities (BM25 pre-population)
3. Runs the document labeler to detect pass gates
4. Runs `KgEmailExtractionAgent` with the detected gates
5. Entities are persisted and indexed after each pass

---

## Open Questions

1. **Pre-population list cap** — how many candidates before context pressure degrades accuracy. Current default: 5 per entity type.

2. **Entity resolution quality** — over-merge vs under-merge rate needs evaluation data.

3. **User-in-the-loop** — entity deduplication (especially orgs/persons) may need a review queue.

4. **UIDVALIDITY change handling** — if the email index is rebuilt, search summaries need re-population.
