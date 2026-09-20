# 2026-09-20 — Founding session

## Format of this log

Each entry is a dated Markdown file in this directory, numbered in session
order (`NNN-short-slug.md`). An entry captures, in this order:

1. The owner's inputs for the session — requirements, real transaction/
   statement examples, corrections to earlier assumptions, decisions made.
   Preserve the owner's own framing/wording where it carries meaning;
   redact real account numbers, balances, and other sensitive values to
   their shape, not actual figures, unless the owner explicitly says
   otherwise.
2. What changed in this crate as a result: types, migrations, extractor.
3. A session summary: what changed, what remains open, so a different
   coding agent can pick this up cold.

## Owner input

This entry transcribes the founding conversation that produced
`epics/002-transaction-type-family-first-use-case.md`, run against the
Dwata restart context in `README.md`/`DEVELOP.md` (backend-first restart,
legacy GUI removed per `epics/001-native-restart-and-legacy-ui-removal.md`).

The direction, as established in that discussion:

- Restart Dwata's data-modeling approach around **model-first,
  need-driven extraction**, replacing the earlier extractor-first approach
  where data was fit into models after the fact.
- A user's actual need (finance, accounting, inventory, care management,
  ...) determines which data models matter. The source of the data (email,
  CSV, bank export, ...) is not architecturally central.
- Broad domain concepts (`Transaction`, `Party`, `Account`, ...) should be
  closed-ish Rust type families: the type itself knows the variations it
  can take and how each variation maps to a database materialization. This
  replaces the earlier idea of an LLM-authored schema agent — schema
  materialization becomes deterministic, driven by which variant is
  recorded as chosen, not generated freeform by an LLM.
- Extraction must be **total**: every source record parses into *some*
  representation of the type, using explicit unresolved/unknown states for
  ambiguity, never a hard parse failure. The existing
  `TransactionParty::Unknown`/`CandidateVendorId` pattern in
  `docs/03-type-driven-financial-extraction.md` is the existing proof this
  works and should generalize to new use cases.
- Derived calculations (totals, balances) must always be correct.
  `shared-types`'s `Transaction.amount: f64` and `Bill.total_amount:
  Option<f64>` are known defects against this — floating point is not safe
  for money arithmetic. New use-case crates must use a decimal-safe `Money`
  newtype instead.
- Extractors should be scoped to what a specific use case needs day to
  day, each with its own system prompt, rather than one general-purpose
  pipeline. The existing reverse-template email pipeline in `dwata-agents`
  becomes, at most, one possible extractor implementation later, not
  infrastructure this work depends on.
- The eventual target architecture is a multi-variant, config-driven schema
  selection system: a `type_configuration`-style table read by both a
  schema reconciler and extractors, likely triggered from the akar GUI
  later. Building that now, before any real use case has been modeled,
  would be premature — this epic exists to produce the first real sample so
  that system can be designed from evidence instead of speculation.
- This work is scoped as one new, isolated crate per use case, under
  `usecases/<slug>/`, each with its own log, types, migrations, and
  extractor — not pushed into `shared-types` yet, and not consolidated
  across use cases until a few exist and real overlap is visible.
- The first use case is the epic owner's own real finances: a small
  business plus family finances, run jointly with a partner. This is a
  sample set of one; the intent is not to solve financial modeling
  generally, but to produce one real, working vertical slice and capture
  the working process as a reusable log so the same structure repeats for
  other use cases later.
- Every session on this epic must log first (owner input, dated,
  preserving framing), then update types, then update schema, then update
  the extractor, then leave a session summary — in that order — so the log
  stays a first-class artifact future sessions and future use-case crates
  can rely on.
- The CLI-based testing approach for this crate is explicitly deferred:
  document it as an open task, do not design or build it, pending the
  owner's input.

Full detail, scope boundaries, and acceptance criteria for this epic are in
`epics/002-transaction-type-family-first-use-case.md`; this entry
summarizes rather than duplicates it.

## What changed

- Created the crate skeleton: `usecases/family-business-finance/Cargo.toml`
  (package `dwata-usecase-family-business-finance`), `src/lib.rs`,
  `README.md`, and this `log/` directory. Registered the crate as a
  workspace member in the root `Cargo.toml`.
- Resolved the schema-integration open decision: this crate will not run
  its own migration runner. Its migration SQL will be added directly as
  the next numbered file(s) in `dwata-api/migrations/` (the existing
  refinery-driven sequence), with no Cargo dependency either direction.
  Recorded in this crate's `README.md` under "Schema-integration
  decision".
- No `Transaction`/`Party`/`Account`/`Money` types, migrations, or
  extractor yet — those depend on the real-use-case session below.

## Session summary

**Done:** crate exists, builds as an empty workspace member, schema-
integration decision recorded, founding conversation logged.

**Open, next session:** hold the real-use-case session with the owner
(epic task 4) — accounts involved, how business vs. family is
distinguished, how salary/partner payments work, what contacts need
tagging, and a redacted real source-material sample. That session's answers
drive the initial `Transaction` variant(s), `Party`/`Account` concepts, and
the `Money` newtype (epic task 5), the migration(s) (task 6), and the first
extractor (task 7). The CLI-testing approach stays explicitly open until
the owner gives input on it (task 8) — do not design or build it
speculatively.
