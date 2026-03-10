CREATE TABLE financial_extraction_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source_type VARCHAR NOT NULL,
    data_source_id VARCHAR NOT NULL,
    template_type VARCHAR NOT NULL,
    template_body TEXT NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'active',
    version INTEGER NOT NULL DEFAULT 1,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    CHECK (template_type IN ('bill', 'transaction')),
    CHECK (status IN ('active', 'superseded', 'disabled'))
);

CREATE INDEX idx_financial_templates_source_status
    ON financial_extraction_templates(data_source_type, data_source_id, status);

CREATE INDEX idx_financial_templates_type_status
    ON financial_extraction_templates(template_type, status);

CREATE TABLE financial_template_variables (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    template_id INTEGER NOT NULL,
    placeholder_name VARCHAR NOT NULL,
    target_field VARCHAR NOT NULL,
    created_at BIGINT NOT NULL,
    FOREIGN KEY(template_id) REFERENCES financial_extraction_templates(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_financial_template_variables_unique
    ON financial_template_variables(template_id, placeholder_name);

CREATE INDEX idx_financial_template_variables_template
    ON financial_template_variables(template_id);

CREATE TABLE financial_template_applicability (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    template_id INTEGER NOT NULL,
    data_source_type VARCHAR NOT NULL,
    data_source_id VARCHAR NOT NULL,
    match_score REAL,
    created_at BIGINT NOT NULL,
    FOREIGN KEY(template_id) REFERENCES financial_extraction_templates(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_financial_template_applicability_unique
    ON financial_template_applicability(template_id, data_source_type, data_source_id);

CREATE INDEX idx_financial_template_applicability_source
    ON financial_template_applicability(data_source_type, data_source_id);
