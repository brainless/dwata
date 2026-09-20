# 2026-09-20 — HDFC statement, first extractor

## Owner input

Owner placed a real HDFC bank statement CSV export at
`usecases/family-business-finance/sample/hdfc_statement.csv` (gitignored,
never committed — see "What changed" below). The file is **not**
redacted: it contains the owner's real name, a real recipient name, real
reference numbers, and real balances. Handled by keeping all real
values out of every committed file (log, README, code, comments) and
gitignoring both the sample and a new `local/` directory holding
real-identity lookup data.

Columns (typical HDFC export shape, one blank line before the header):
`Date, Narration, Chq./Ref.No., Value Dt, Withdrawal Amt., Deposit Amt.,
Closing Balance`.

From the sample's 7 data rows, walked through with the owner:

1. A debit narrated `...-TPT-SALARY JUNE-<OWNER NAME>` — confirmed: salary
   paid to the owner from the (legacy, reused) business current account.
2. A debit narrated `NEFT DR-...-<NAME>-NETBANK...-SALARY JUNE` to a
   different name — confirmed: the owner's wife. Owner noted the salary
   narration *structure* is similar across recipients (not a one-off
   format for this transaction), i.e. more family members' salaries will
   look like this same shape with a different name.
3. A credit narrated `INW <ref> USD<amount>@<rate>` — confirmed: an
   inward foreign remittance, a client paying for consulting. Owner wants
   only the settled INR value tracked (the `Deposit Amt.` column, already
   INR) — no need to carry the original USD amount or FX rate in the
   type.
4. Two debits narrated `...DPO<ref> CGST` / `...DPO<ref> SGST` right after
   the inward remittance (GST charged on the receipt) — owner: fold into
   `Unresolved`, no dedicated `TransactionKind` variant for now.
5. Debits narrated `...INSTAALERTCHG...SMS-<ref>` — recurring SMS-alert
   bank fees. Owner: fold into `Unresolved`, same as GST.

This resolves the account-identity question from log `002` for this
particular file: the statement is the **business current account** (it
pays out salaries and receives the consultancy inward remittance), not one
of the two personal savings accounts.

## What changed

- Added `usecases/*/sample/` and `usecases/*/local/` to the repo-root
  `.gitignore`. `sample/` holds owner-provided raw source files (like this
  CSV); `local/` holds derived data that names real people (the party
  registry below). Neither is ever committed. Confirmed with
  `git check-ignore`.
- Added `party_registry.rs`: `PartyRegistry`/`PartyRegistryEntry`, loaded
  from local JSON, resolving a name fragment appearing in a narration to a
  `Party`. Exists specifically so real names never need to be hardcoded in
  committed source — the registry is data, loaded at runtime from
  `local/known_parties.json` (gitignored, not committed), not code.
- Added `extractors/hdfc_bank_csv.rs`: `parse_hdfc_statement_csv(csv,
  account_slug, registry) -> Vec<Transaction>`. Total by construction:
  every row with a valid date and a withdrawal-or-deposit amount produces
  a `Transaction`; anything not matched by a known narration pattern
  becomes `TransactionKind::Unresolved` with the raw narration preserved,
  never dropped, never a parse failure. Money is parsed as integer paise
  directly from the decimal string (no float in the path). Patterns
  implemented, matching only what's evidenced in the sample:
  - `SALARY` anywhere in the narration → `Salary`, recipient resolved via
    `PartyRegistry` (falls back to `Party::Unknown` for an unrecognized
    name rather than failing).
  - Narration starts with `INW ` and contains `USD` → `ConsultancyIncome`
    (client left `Party::Unknown` — not named in this sample's
    narration), `BusinessContext::SoftwareConsultancy`.
  - Everything else (including the GST and SMS-fee rows) →
    `TransactionKind::Unresolved`, `BusinessContext::Unresolved`, per the
    owner's fold-into-Unresolved decisions above.
  - `Salary` rows are given `BusinessContext::Unresolved`, not guessed —
    see open item below.
- Added `local/known_parties.json` (gitignored): registry entries for the
  owner and their wife, built from the names seen in the sample file, so
  the extractor is actually runnable against real data locally.
- Unit tests use synthetic narrations with placeholder names (`OWNER
  NAME`, `SPOUSE NAME`), never the real sample content, so they run in CI
  without the gitignored files present.

## Session summary

**Done:** first extractor built and tested against the real HDFC statement
shape (8 passing unit tests on synthetic data mirroring the real
narrations); `Salary`, `ConsultancyIncome`, and `Unresolved` (covering GST
and SMS fees) are all reachable; nothing is dropped or fails to parse.
Workspace builds and `cargo fmt --check` passes.

**Open, next session:**

- **`Salary` → `BusinessContext`**: currently always `Unresolved` because
  a bank row alone doesn't say whether a given salary payment should be
  attributed to the software-consultancy business (as the funding source)
  or treated as neutral/family. Needs owner input on how they want this
  attributed, if at all — possibly it should just stay `Unresolved` by
  design rather than get a per-row guess.
- **Mother's and business partner's salary/expense narrations** haven't
  been seen yet — only the owner's and the wife's. Expect
  `party_registry` entries and possibly narration-pattern tweaks once
  those show up in a sample.
- **`FamilyBusinessExpense`, `PersonalOrFamilyExpense`,
  `AccountTransfer`, `CreditCardCharge`** are defined in the type family
  but not yet reachable from this extractor — no sample row matched them
  yet. Not implemented speculatively; wait for real rows.
- **Credit-card PDF and payment-app CSV/Excel samples** still needed for
  those two source formats (epic's source-material list).
- **Persisting to the database** (inserting parsed `Transaction`s into
  `ffb_transactions` etc.) hasn't been wired up yet — this session only
  produces in-memory `Transaction` values from a CSV string.
- CLI-testing approach remains explicitly open per the epic; not designed.
