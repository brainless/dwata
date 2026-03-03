-- Enforce transaction status model:
-- financial_transactions.status can only be paid/cancelled/refunded.
-- Legacy pending/overdue rows are dropped during migration.

PRAGMA foreign_keys=OFF;

ALTER TABLE financial_transactions RENAME TO financial_transactions_old;

CREATE TABLE financial_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source_type VARCHAR NOT NULL,
    data_source_id VARCHAR NOT NULL,
    amount REAL NOT NULL,
    currency VARCHAR NOT NULL,
    transaction_date VARCHAR NOT NULL,
    category VARCHAR,
    source_vendor_id INTEGER,
    destination_vendor_id INTEGER,
    status VARCHAR NOT NULL,
    source_file VARCHAR,
    requires_review BOOLEAN NOT NULL DEFAULT false,
    extracted_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    notes VARCHAR,
    transaction_reference VARCHAR,
    CHECK (status IN ('paid', 'cancelled', 'refunded'))
);

INSERT INTO financial_transactions (
    id, data_source_type, data_source_id, amount, currency, transaction_date, category,
    source_vendor_id, destination_vendor_id, status, source_file, requires_review,
    extracted_at, created_at, updated_at, notes, transaction_reference
)
SELECT
    id, data_source_type, data_source_id, amount, currency, transaction_date, category,
    source_vendor_id, destination_vendor_id, status, source_file, requires_review,
    extracted_at, created_at, updated_at, notes, transaction_reference
FROM financial_transactions_old
WHERE status IN ('paid', 'cancelled', 'refunded');

DROP TABLE financial_transactions_old;

CREATE UNIQUE INDEX idx_financial_transactions_source_reference_unique
    ON financial_transactions(data_source_type, data_source_id, transaction_reference)
    WHERE transaction_reference IS NOT NULL;

CREATE UNIQUE INDEX idx_financial_transactions_source_fallback_unique
    ON financial_transactions(data_source_type, data_source_id, amount, transaction_date);

CREATE INDEX idx_financial_transactions_date
    ON financial_transactions(transaction_date DESC, id DESC);
CREATE INDEX idx_financial_transactions_category_date
    ON financial_transactions(category, transaction_date DESC);
CREATE INDEX idx_financial_transactions_status_date
    ON financial_transactions(status, transaction_date DESC);
CREATE INDEX idx_financial_transactions_source_vendor
    ON financial_transactions(source_vendor_id);
CREATE INDEX idx_financial_transactions_destination_vendor
    ON financial_transactions(destination_vendor_id);

PRAGMA foreign_keys=ON;
