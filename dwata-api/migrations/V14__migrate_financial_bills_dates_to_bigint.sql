-- V13 already created financial_bills with parsed date columns as VARCHAR on existing installs.
-- This migration converts parsed date columns to BIGINT UTC epoch milliseconds while preserving
-- raw date strings and indexes/constraints.

PRAGMA foreign_keys=OFF;

ALTER TABLE financial_bills RENAME TO financial_bills_old;

CREATE TABLE financial_bills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source_type VARCHAR NOT NULL,
    data_source_id VARCHAR NOT NULL,
    template_id INTEGER,
    document_type VARCHAR NOT NULL,
    status VARCHAR NOT NULL,
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
    issuer_vendor_id, document_reference, total_amount, currency,
    issued_date_raw, issued_date,
    due_date_raw, due_date,
    billing_period_start_raw, billing_period_start,
    billing_period_end_raw, billing_period_end,
    created_at, updated_at
)
SELECT
    id, data_source_type, data_source_id, template_id, document_type, status,
    issuer_vendor_id, document_reference, total_amount, currency,
    issued_date_raw,
    CASE
        WHEN issued_date IS NULL OR TRIM(CAST(issued_date AS TEXT)) = '' THEN NULL
        WHEN typeof(issued_date) = 'integer' THEN issued_date
        ELSE COALESCE(
            CAST(strftime('%s', issued_date) AS INTEGER) * 1000,
            CAST(strftime('%s', issued_date || ' 00:00:00', 'utc') AS INTEGER) * 1000
        )
    END AS issued_date,
    due_date_raw,
    CASE
        WHEN due_date IS NULL OR TRIM(CAST(due_date AS TEXT)) = '' THEN NULL
        WHEN typeof(due_date) = 'integer' THEN due_date
        ELSE COALESCE(
            CAST(strftime('%s', due_date) AS INTEGER) * 1000,
            CAST(strftime('%s', due_date || ' 00:00:00', 'utc') AS INTEGER) * 1000
        )
    END AS due_date,
    billing_period_start_raw,
    CASE
        WHEN billing_period_start IS NULL OR TRIM(CAST(billing_period_start AS TEXT)) = '' THEN NULL
        WHEN typeof(billing_period_start) = 'integer' THEN billing_period_start
        ELSE COALESCE(
            CAST(strftime('%s', billing_period_start) AS INTEGER) * 1000,
            CAST(strftime('%s', billing_period_start || ' 00:00:00', 'utc') AS INTEGER) * 1000
        )
    END AS billing_period_start,
    billing_period_end_raw,
    CASE
        WHEN billing_period_end IS NULL OR TRIM(CAST(billing_period_end AS TEXT)) = '' THEN NULL
        WHEN typeof(billing_period_end) = 'integer' THEN billing_period_end
        ELSE COALESCE(
            CAST(strftime('%s', billing_period_end) AS INTEGER) * 1000,
            CAST(strftime('%s', billing_period_end || ' 00:00:00', 'utc') AS INTEGER) * 1000
        )
    END AS billing_period_end,
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

CREATE INDEX idx_financial_bills_template
    ON financial_bills(template_id);

PRAGMA foreign_keys=ON;
