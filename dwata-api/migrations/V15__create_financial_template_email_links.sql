CREATE TABLE financial_template_email_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    template_id INTEGER NOT NULL,
    email_id INTEGER NOT NULL,
    match_score REAL,
    created_at BIGINT NOT NULL,
    FOREIGN KEY(template_id) REFERENCES financial_extraction_templates(id) ON DELETE CASCADE,
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_financial_template_email_links_unique
    ON financial_template_email_links(template_id, email_id);

CREATE INDEX idx_financial_template_email_links_email
    ON financial_template_email_links(email_id);
