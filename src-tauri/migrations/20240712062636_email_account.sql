-- This table stores the email account details

CREATE TABLE email_account
(
    id                          INTEGER PRIMARY KEY,

    provider                    VARCHAR(30) NOT NULL,
    email_address               VARCHAR(255) NOT NULL,
    password                    VARCHAR(255),
    oauth2_token_id             INTEGER,

    created_at                  DATETIME NOT NULL,
    modified_at                 DATETIME,

    FOREIGN KEY (oauth2_token_id) REFERENCES oauth2_token (id) ON DELETE SET NULL,
    UNIQUE(provider, email_address) ON CONFLICT ABORT
);

-- This table stores the mailboxes of the email account

CREATE TABLE mailbox
(
    id                          INTEGER PRIMARY KEY,

    email_account_id            INTEGER NOT NULL,
    name                        VARCHAR(30) NOT NULL,
    storage_path                VARCHAR(255),
    messages                    INTEGER,
    uid_next                    INTEGER,
    uid_validity                INTEGER,

    created_at                  DATETIME NOT NULL,
    modified_at                 DATETIME,

    FOREIGN KEY (email_account_id) REFERENCES email_account (id) ON DELETE CASCADE
);
