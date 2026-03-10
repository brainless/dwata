-- Fix financial_transactions foreign key that incorrectly references financial_bills_old
-- (a leftover from the V17 migration rename step that got baked into the stored schema).

PRAGMA foreign_keys=OFF;

ALTER TABLE financial_transactions RENAME TO financial_transactions_fk_fix_old;

CREATE TABLE financial_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source_type VARCHAR NOT NULL,
    data_source_id VARCHAR NOT NULL,
    amount REAL NOT NULL,
    currency VARCHAR NOT NULL,
    transaction_date_raw VARCHAR,
    transaction_date BIGINT,
    status VARCHAR NOT NULL,
    payer_vendor_id INTEGER,
    payee_vendor_id INTEGER,
    transaction_reference VARCHAR,
    bill_id INTEGER,
    source_file VARCHAR,
    requires_review BOOLEAN NOT NULL DEFAULT false,
    extracted_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY(bill_id) REFERENCES financial_bills(id) ON DELETE SET NULL,
    CHECK (status IN ('paid', 'cancelled', 'refunded'))
);

INSERT INTO financial_transactions
SELECT * FROM financial_transactions_fk_fix_old;

DROP TABLE financial_transactions_fk_fix_old;

PRAGMA foreign_keys=ON;
