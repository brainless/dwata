-- V6: Knowledge Graph tables
--
-- All KG entity tables are created here. Using CREATE TABLE IF NOT EXISTS
-- so re-running against a dev database that already has hand-created tables
-- does not error out.

CREATE TABLE IF NOT EXISTS locations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT,                       -- named place, e.g. "Delhi Airport"
    address_line1   TEXT,
    address_line2   TEXT,
    city            TEXT,
    region          TEXT,
    country_code    TEXT,                       -- ISO 3166-1 alpha-2 or alpha-3
    postal_code     TEXT,
    search_summary  TEXT,                       -- BM25-indexed summary for entity pre-population
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS organisations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    description     TEXT,
    industry        TEXT,
    email           TEXT,                       -- primary billing/contact email
    location_id     INTEGER REFERENCES locations(id),
    website         TEXT,
    linkedin_url    TEXT,
    search_summary  TEXT,                       -- BM25-indexed summary for entity pre-population
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

-- Junction table: one row per (organisation, role) pair.
-- An organisation can play multiple roles (e.g. both Business and PaymentPlatform).
CREATE TABLE IF NOT EXISTS organisation_roles (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    organisation_id     INTEGER NOT NULL REFERENCES organisations(id) ON DELETE CASCADE,
    role                TEXT NOT NULL,          -- kebab-case OrganisationRole variant
    UNIQUE(organisation_id, role)
);

CREATE INDEX IF NOT EXISTS idx_organisation_roles_org ON organisation_roles(organisation_id);

CREATE TABLE IF NOT EXISTS persons (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id        INTEGER REFERENCES emails(id),
    name            TEXT NOT NULL,
    email           TEXT,
    phone           TEXT,
    organisation_id INTEGER REFERENCES organisations(id),
    search_summary  TEXT,                       -- BM25-indexed summary for entity pre-population
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_persons_email ON persons(email) WHERE email IS NOT NULL;

-- Social/professional profile links for a person.
CREATE TABLE IF NOT EXISTS contact_links (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    person_id   INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    link_type   TEXT NOT NULL,                  -- linkedin | github | twitter | personal | other
    url         TEXT NOT NULL,
    label       TEXT,
    is_primary  BOOLEAN NOT NULL DEFAULT 0,
    is_verified BOOLEAN NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contact_links_person ON contact_links(person_id);

CREATE TABLE IF NOT EXISTS subscriptions (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    organisation_id         INTEGER REFERENCES organisations(id),
    service_name            TEXT NOT NULL,
    plan_name               TEXT,
    billing_cycle           TEXT,               -- snake_case BillingCycle variant
    amount                  REAL,
    currency                TEXT,
    next_billing_date_raw   TEXT,
    next_billing_date       INTEGER,            -- UTC ms
    start_date_raw          TEXT,
    start_date              INTEGER,            -- UTC ms
    source_email_id         INTEGER REFERENCES emails(id),
    created_at              INTEGER NOT NULL,
    updated_at              INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS orders (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    organisation_id INTEGER REFERENCES organisations(id),
    order_reference TEXT,
    order_date_raw  TEXT,
    order_date      INTEGER,                    -- UTC ms
    status          TEXT,                       -- snake_case OrderStatus variant
    total_amount    REAL,
    currency        TEXT,
    items           TEXT,                       -- JSON: array of {name, quantity?, unit_price?}
    tracking_number TEXT,
    transaction_id  INTEGER,                    -- informational FK to transactions table
    source_email_id INTEGER REFERENCES emails(id),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    description     TEXT,
    event_date_raw  TEXT,
    event_date      INTEGER,                    -- UTC ms
    location_id     INTEGER REFERENCES locations(id),
    attendees       TEXT,                       -- JSON: array of person IDs (integers)
    project_id      INTEGER,
    task_id         INTEGER,
    source_email_id INTEGER REFERENCES emails(id),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
