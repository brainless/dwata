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
    issued_date VARCHAR,
    due_date_raw VARCHAR,
    due_date VARCHAR,
    billing_period_start_raw VARCHAR,
    billing_period_start VARCHAR,
    billing_period_end_raw VARCHAR,
    billing_period_end VARCHAR,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY(template_id) REFERENCES financial_extraction_templates(id) ON DELETE SET NULL,
    CHECK (document_type IN ('invoice', 'bill', 'bank-statement', 'receipt', 'tax-document', 'payment-confirmation')),
    CHECK (status IN ('received', 'unpaid', 'paid', 'overdue', 'cancelled'))
);

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

CREATE TABLE financial_bill_subjects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bill_id INTEGER NOT NULL,
    kind VARCHAR NOT NULL,
    value VARCHAR NOT NULL,
    masked_value VARCHAR,
    is_primary BOOLEAN NOT NULL DEFAULT false,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY(bill_id) REFERENCES financial_bills(id) ON DELETE CASCADE,
    CHECK (kind IN ('phone-number', 'account-number', 'policy-number', 'meter-number', 'subscription-id', 'contract-id', 'other'))
);

CREATE INDEX idx_financial_bill_subjects_bill
    ON financial_bill_subjects(bill_id);

CREATE UNIQUE INDEX idx_financial_bill_subjects_primary_per_bill
    ON financial_bill_subjects(bill_id)
    WHERE is_primary = true;
