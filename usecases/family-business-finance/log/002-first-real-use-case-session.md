# 2026-09-20 — First real-use-case session

## Owner input

(Shape preserved, no real balances/account numbers/institution names given
or recorded — the owner described accounts and flows in general terms.)

**1. Accounts involved**

- Two personal savings accounts.
- One business (current) account. This account is old — it was originally
  opened for a *previous, unrelated* business — and is currently being
  reused to pay for the family-run business's expenses.
- Two credit cards.
- The owner's mother has her own account; she takes a salary from the
  business account and pays some expenses from it.
- The owner's wife also takes a salary from the business account and pays
  some expenses from it.
- The owner's business partner also receives money from the business
  account and spends it on the family-run business.
- Future: a dedicated bank account will be opened specifically for the
  family-run business (separate from the reused legacy business account).

**2. Business vs. family distinction**

- Currently mixed within the same account — not cleanly separated by
  account alone. The business (current) account mostly pays out salaries
  because it's the older account being repurposed.
- Almost all transactions carry a comment/note in the bank record.
- The owner's original business is software consultancy — majority of
  income comes from there.
- Salaries paid to the owner and family members get invested into the
  family-run business, which is pig and chicken farming.
- Most digital transactions have comments, or the merchant account
  involved is already known/classifiable by the owner.

**3. Salary / partner payments**

- Salaries are mostly fixed transfers.
- Bank records (CSV/Excel exports) have clear comments identifying them.

**4. Contacts to tag**

- The list of people/organisations involved is growing but mostly known
  already (family members, business partner, consultancy clients, farm
  vendors/merchants).

**5. Source material format**

- Bank exports: CSV.
- Credit card bills: PDF.
- Digital payment app exports: mostly CSV/Excel.
- No redacted sample file has been shared yet — needed before the
  extractor can be built against a real column layout. Flagged as open
  below.

## What changed

Defined the initial type family in `src/`, reflecting the above:

- `money.rs` — `Money` (integer minor-units + `CurrencyCode`, no `f64`).
- `party.rs` — `Party` (`Person` / `Organisation` / `Unknown`),
  `PersonRelationship` (`Owner`, `Spouse`, `Mother`, `BusinessPartner`,
  `OtherFamily`), `OrganisationRole` (`ConsultancyClient`, `FarmVendor`,
  `Merchant`, `Other`). `Unknown` exists for totality when a party can't be
  resolved from a raw description.
- `account.rs` — `Account`, `AccountKind` (`PersonalSavings`,
  `BusinessCurrent`, `CreditCard`), `AccountStatus` (`Active`, `Legacy` —
  the reused business account is modeled `Legacy` since it predates this
  use case). No dedicated family-run-business account kind is added yet
  since none exists — the future account is a follow-up, not modeled
  speculatively.
- `transaction.rs` — `Transaction`, `TransactionDirection`
  (`Credit`/`Debit`), `BusinessContext` (`SoftwareConsultancy`,
  `FamilyFarm`, `Personal`, `Unresolved`), `TransactionKind`
  (`ConsultancyIncome`, `Salary`, `FamilyBusinessExpense`,
  `PersonalOrFamilyExpense`, `AccountTransfer`, `CreditCardCharge`,
  `Unresolved`) — `Unresolved` is the totality escape hatch for anything
  the extractor can't classify yet, carrying the raw description rather
  than dropping the record.

Migration `dwata-api/migrations/V13__create_family_business_finance_tables.sql`
materializes `ffb_accounts`, `ffb_parties`, `ffb_transactions` (prefix `ffb_`
to namespace this use case's tables in the shared database), per the
schema-integration decision in this crate's `README.md`.

No extractor yet — see open items below.

## Session summary

**Done:** owner's real-use-case answers logged; initial `Money`/`Party`/
`Account`/`Transaction` type family defined in this crate reflecting two
personal savings accounts, one legacy-but-reused business current account,
two credit cards, salaried family members (mother, wife) and a business
partner drawing from the business account, and the software-consultancy
income vs. family-farm-business expense split; migration `V13` added to
`dwata-api/migrations/` materializing these types; workspace builds.

**Open, next session:**

- Get a **redacted sample** of at least one bank CSV export (column
  headers plus a couple of shape-only rows — no real amounts/account
  numbers) so the extractor can be built against the actual column layout,
  per epic task 7. Credit-card PDF and payment-app CSV/Excel samples would
  help too but bank CSV is the priority to unblock a first extractor.
- The `TransactionKind`/`BusinessContext` variants above are a first pass
  from a verbal description, not yet checked against a real row — expect
  to adjust once a real sample is seen (e.g. whether "family business
  expense" needs to distinguish paid directly by the owner vs. by a
  salaried family member vs. by the partner).
- CLI-testing approach remains explicitly open per the epic; not designed.
