CREATE TABLE email_label_associations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id INTEGER NOT NULL,
    label_id INTEGER NOT NULL,
    created_at BIGINT NOT NULL,
    UNIQUE(email_id, label_id),
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE,
    FOREIGN KEY(label_id) REFERENCES email_labels(id) ON DELETE CASCADE
);

CREATE INDEX idx_email_label_assoc_email ON email_label_associations(email_id);
CREATE INDEX idx_email_label_assoc_label ON email_label_associations(label_id);
