# Type-Driven Financial Extraction

## Purpose

This document defines the engineering north star for Dwata financial extraction.

Dwata should prefer **parse into trusted types** over parse-then-validate.  
Reference idea: [Parse, don’t validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/).

## Core Principle

If extraction succeeds, the result should already be a valid domain object.

- No partially-valid transaction objects.
- No "best effort" fields that require downstream cleanup.
- Unknowns are represented explicitly in types.
- Failure to parse required semantics is a hard extraction failure.

## Domain Model Rules

`FinancialTransaction` is the canonical output contract.

- A transaction always has two parties: `payer` and `payee`.
- Party identity is explicit:
  - `SelfEntity`
  - `KnownVendorId(i64)`
  - `CandidateVendorId(i64)`
  - `Unknown`
- User-enrichment is first-class:
  - `enrichment_status` tracks lifecycle.
  - `unresolved_items` tracks what still needs user input.
- Confidence scoring is intentionally excluded from the core model.

## Extraction Strategy

Dwata uses a reverse-template pipeline as the primary extraction architecture.

1. Cluster similar sender documents.
2. Build template(s) by diffing repeated documents.
3. Map template variables to typed financial fields.
4. Parse each document using the typed template contract.
5. Emit `FinancialTransaction` only when required semantics are present.

Regex pattern generation/storage from the old extractor path is deprecated and removed.

## Deterministic Success Criteria

Extraction is considered successful only if:

- Amount parses to `f64`.
- Document type and status parse to known enums.
- `payer` and `payee` are present as typed `TransactionParty`.
- Date/currency/reference rules for the active extractor profile are satisfied.

Otherwise, extraction fails and should be retried with improved template/parser logic or user guidance.

## User-In-The-Loop Roadmap

Dwata is designed for progressive enrichment, not silent guessing.

Future features should include:

1. Review queue generated from `unresolved_items`.
2. User confirmation workflow for `CandidateVendorId`.
3. Explicit "set as SelfEntity" actions for party endpoints.
4. Incremental re-resolution after user actions.
5. Template learning from confirmed user corrections.

## Implementation Guardrails

When adding extraction features:

- Prefer enum/newtype expansion over optional free-text fields.
- Avoid adding "temporary" ambiguous states that bypass types.
- Keep parser boundaries strict: parsing and validation should not be separate phases for domain invariants.
- Keep backward-compatibility bridges outside core domain types (adapters at DB/API edges only).

## What To Avoid

- Reintroducing confidence fields in core transaction types.
- Reintroducing regex-first extraction as primary strategy.
- Storing unresolved semantics as plain strings when a typed variant can represent them.
- Letting downstream jobs "fix" invalid transactions after insertion.

## Ownership

This document is the reference for financial extraction design decisions.
Any change to `shared-types/src/financial.rs` should be evaluated against this document.
