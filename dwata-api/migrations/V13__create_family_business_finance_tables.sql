-- Materializes the dwata-usecase-family-business-finance crate's type
-- family (usecases/family-business-finance/src/). Tables are prefixed
-- `ffb_` to namespace this use case within the shared database, per the
-- schema-integration decision recorded in that crate's README.md.

CREATE TABLE ffb_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug VARCHAR NOT NULL UNIQUE,
    kind VARCHAR NOT NULL CHECK (kind IN ('personal_savings', 'business_current', 'credit_card')),
    status VARCHAR NOT NULL CHECK (status IN ('active', 'legacy')),
    holder_relationship VARCHAR NOT NULL CHECK (holder_relationship IN ('owner', 'spouse', 'mother', 'business_partner', 'other_family')),
    currency VARCHAR NOT NULL DEFAULT 'INR',
    note VARCHAR,
    created_at BIGINT NOT NULL
);

CREATE TABLE ffb_parties (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind VARCHAR NOT NULL CHECK (kind IN ('person', 'organisation', 'unknown')),
    name VARCHAR,
    person_relationship VARCHAR CHECK (person_relationship IN ('owner', 'spouse', 'mother', 'business_partner', 'other_family')),
    organisation_role VARCHAR CHECK (organisation_role IN ('consultancy_client', 'farm_vendor', 'merchant', 'other')),
    raw_label VARCHAR,
    created_at BIGINT NOT NULL
);

CREATE TABLE ffb_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES ffb_accounts(id),
    occurred_on VARCHAR NOT NULL,
    direction VARCHAR NOT NULL CHECK (direction IN ('credit', 'debit')),
    amount_minor_units BIGINT NOT NULL,
    currency VARCHAR NOT NULL DEFAULT 'INR',
    business_context VARCHAR NOT NULL CHECK (business_context IN ('software_consultancy', 'family_farm', 'personal', 'unresolved')),
    kind VARCHAR NOT NULL CHECK (kind IN ('consultancy_income', 'salary', 'family_business_expense', 'personal_or_family_expense', 'account_transfer', 'credit_card_charge', 'unresolved')),
    counterparty_party_id INTEGER REFERENCES ffb_parties(id),
    counterparty_account_id INTEGER REFERENCES ffb_accounts(id),
    raw_description VARCHAR NOT NULL,
    source VARCHAR NOT NULL CHECK (source IN ('bank_csv', 'credit_card_pdf', 'payment_app_export')),
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_ffb_transactions_account_id ON ffb_transactions(account_id);
CREATE INDEX idx_ffb_transactions_occurred_on ON ffb_transactions(occurred_on);
CREATE INDEX idx_ffb_transactions_business_context ON ffb_transactions(business_context);
