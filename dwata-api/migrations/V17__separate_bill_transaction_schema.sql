-- Separate Bill and Transaction:
-- - Transaction: compact bank ledger entry
-- - Bill: rich document with category
-- - Transaction can optionally link to Bill via bill_id

PRAGMA foreign_keys=OFF;

-- Migrate transactions table
ALTER TABLE financial_transactions RENAME TO financial_transactions_old;

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

INSERT INTO financial_transactions (
    id, data_source_type, data_source_id, amount, currency,
    transaction_date_raw, transaction_date,
    status, payer_vendor_id, payee_vendor_id,
    transaction_reference, bill_id,
    source_file, requires_review, extracted_at, created_at, updated_at
)
SELECT
    id, data_source_type, data_source_id, amount, currency,
    NULL AS transaction_date_raw,
    CASE
        WHEN transaction_date IS NULL OR TRIM(CAST(transaction_date AS TEXT)) = '' THEN NULL
        WHEN typeof(transaction_date) = 'integer' THEN transaction_date
        ELSE COALESCE(
            CAST(strftime('%s', transaction_date) AS INTEGER) * 1000,
            CAST(strftime('%s', transaction_date || ' 00:00:00', 'utc') AS INTEGER) * 1000
        )
    END AS transaction_date,
    status,
    source_vendor_id AS payer_vendor_id,
    destination_vendor_id AS payee_vendor_id,
    transaction_reference,
    NULL AS bill_id,
    source_file,
    requires_review,
    extracted_at,
    created_at,
    updated_at
FROM financial_transactions_old;

DROP TABLE financial_transactions_old;

CREATE UNIQUE INDEX idx_financial_transactions_source_reference_unique
    ON financial_transactions(data_source_type, data_source_id, transaction_reference)
    WHERE transaction_reference IS NOT NULL;

CREATE UNIQUE INDEX idx_financial_transactions_source_fallback_unique
    ON financial_transactions(data_source_type, data_source_id, amount, transaction_date);

CREATE INDEX idx_financial_transactions_date
    ON financial_transactions(transaction_date DESC, id DESC);
CREATE INDEX idx_financial_transactions_status_date
    ON financial_transactions(status, transaction_date DESC);
CREATE INDEX idx_financial_transactions_payer_vendor
    ON financial_transactions(payer_vendor_id);
CREATE INDEX idx_financial_transactions_payee_vendor
    ON financial_transactions(payee_vendor_id);
CREATE INDEX idx_financial_transactions_bill
    ON financial_transactions(bill_id);

-- Migrate bills table: add category
ALTER TABLE financial_bills RENAME TO financial_bills_old;

CREATE TABLE financial_bills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source_type VARCHAR NOT NULL,
    data_source_id VARCHAR NOT NULL,
    template_id INTEGER,
    document_type VARCHAR NOT NULL,
    status VARCHAR NOT NULL,
    category VARCHAR,
    issuer_vendor_id INTEGER,
    document_reference VARCHAR,
    total_amount REAL,
    currency VARCHAR,
    issued_date_raw VARCHAR,
    issued_date BIGINT,
    due_date_raw VARCHAR,
    due_date BIGINT,
    billing_period_start_raw VARCHAR,
    billing_period_start BIGINT,
    billing_period_end_raw VARCHAR,
    billing_period_end BIGINT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY(template_id) REFERENCES financial_extraction_templates(id) ON DELETE SET NULL,
    CHECK (document_type IN ('invoice', 'bill', 'bank-statement', 'receipt', 'tax-document', 'payment-confirmation')),
    CHECK (status IN ('received', 'unpaid', 'paid', 'overdue', 'cancelled'))
);

INSERT INTO financial_bills (
    id, data_source_type, data_source_id, template_id, document_type, status,
    category, issuer_vendor_id, document_reference, total_amount, currency,
    issued_date_raw, issued_date,
    due_date_raw, due_date,
    billing_period_start_raw, billing_period_start,
    billing_period_end_raw, billing_period_end,
    created_at, updated_at
)
SELECT
    id, data_source_type, data_source_id, template_id, document_type, status,
    NULL AS category,
    issuer_vendor_id, document_reference, total_amount, currency,
    issued_date_raw, issued_date,
    due_date_raw, due_date,
    billing_period_start_raw, billing_period_start,
    billing_period_end_raw, billing_period_end,
    created_at, updated_at
FROM financial_bills_old;

DROP TABLE financial_bills_old;

CREATE UNIQUE INDEX idx_financial_bills_source_reference_unique
    ON financial_bills(data_source_type, data_source_id, document_reference)
    WHERE document_reference IS NOT NULL;

CREATE UNIQUE INDEX idx_financial_bills_source_fallback_unique
    ON financial_bills(data_source_type, data_source_id, total_amount, due_date);

CREATE INDEX idx_financial_bills_due_date
    ON financial_bills(due_date DESC, id DESC);

CREATE INDEX idx_financial_bills_status_due_date
    ON financial_bills(status, due_date DESC);

CREATE INDEX idx_financial_bills_category
    ON financial_bills(category);

CREATE INDEX idx_financial_bills_template
    ON financial_bills(template_id);

PRAGMA foreign_keys=ON;
