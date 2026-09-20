# dwata-usecase-family-business-finance

First instance of the "use-case crate" pattern described in
`epics/002-transaction-type-family-first-use-case.md`: a model-first,
need-driven `Transaction`/`Party`/`Account` type family for the epic owner's
own small business plus joint family finances.

This crate is the reference example for the pattern — future use cases
(inventory, care management, ...) should follow the same shape: their own
crate under `usecases/<slug>/`, their own `log/`, their own types/schema/
extractor.

## Status

`Money`/`Party`/`Account`/`Transaction` type family defined in `src/`
(reflecting the 2026-09-20 real-use-case session — two personal savings
accounts, one legacy-but-reused business current account, two credit
cards, salaried family members and a business partner drawing from the
business account, software-consultancy income vs. family-farm-business
expenses) and materialized via
`dwata-api/migrations/V13__create_family_business_finance_tables.sql`.

A first extractor (`extractors::hdfc_bank_csv`) parses HDFC bank statement
CSV exports into `Transaction`s, built and tested against a real (never
committed) sample export. It is not yet wired up to persist into the
database, and several `TransactionKind` variants aren't reachable from it
yet (no sample row exercised them). See `log/003-...` for exactly what's
implemented and what's open.

## Real data stays local, never committed

Two gitignored directories hold real, non-redacted data this crate's code
and tests must never depend on being present, and must never have its
content copied into a committed file:

- `sample/` — raw source exports the owner drops in (e.g. a bank CSV).
- `local/` — derived data that names real people, e.g.
  `known_parties.json`, the registry `party_registry::PartyRegistry`
  loads at runtime to resolve names seen in narrations to a `Party`.
  Real names live only in this file, never in source.

Both are listed in the repo-root `.gitignore` under `usecases/*/sample/`
and `usecases/*/local/`. Tests use synthetic placeholder data so they pass
without either directory present.

## Schema-integration decision

`dwata-api` owns the single SQLite database and its migration runner
(`dwata-api/src/database/mod.rs`, using `refinery::embed_migrations!` over
the `dwata-api/migrations/` directory, plain numbered `V{n}__name.sql`
files).

Decision: this crate does **not** introduce a second migration runner, and
neither crate takes a Cargo dependency on the other for schema purposes.
Instead, this crate's migrations are authored as plain SQL and placed
directly into `dwata-api/migrations/` as the next `V{n}__...sql` file(s) in
that single numbered sequence, so `dwata-api`'s existing refinery runner
picks them up unmodified. This crate's log records exactly which migration
file(s) belong to it, so the connection back to this use case stays
traceable without `dwata-api` needing to know this crate exists at compile
time.

This keeps with the epic's instruction not to build the config-driven
`type_configuration`/schema-reconciler system yet — no new migration
mechanism, just one more file in the existing one.

Rust type definitions for this use case's `Transaction`/`Party`/`Account`
variants and the `Money` newtype live in this crate's `src/`, independent of
where the SQL that materializes them is registered.

## CLI testing approach — open

Not designed or implemented as part of this epic. The epic owner will give
input on what this CLI should do (e.g. run the extractor against a sample
file and show the parsed/unresolved rows) before it is built. Tracked as an
open task; see `epics/002-transaction-type-family-first-use-case.md` and
`log/`.

## Log

`log/` holds dated session entries per the epic's working method: owner
inputs first (redacted to shape, not real figures/account numbers), then
what changed in types/schema/extractor, then a session summary for the next
session to pick up from.
