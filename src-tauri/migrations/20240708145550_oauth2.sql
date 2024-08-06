-- This table stores the OAuth2 app for third party services

CREATE TABLE oauth2_app
(
    id                          INTEGER PRIMARY KEY,

    provider                    VARCHAR(30) NOT NULL,
    client_id                   VARCHAR(255) NOT NULL,
    client_secret               VARCHAR(255),

    created_at                  DATETIME NOT NULL,
    modified_at                 DATETIME
);

-- This table stores the OAuth2 tokens for any OAuth2 app

CREATE TABLE oauth2_token
(
    id                          INTEGER PRIMARY KEY,

    oauth2_app_id               INTEGER NOT NULL,

    authorization_code          VARCHAR(255) NOT NULL,
    access_token                VARCHAR(255) NOT NULL,
    refresh_token               VARCHAR(255),
    identifier                  VARCHAR(255) NOT NULL,
    handle                      VARCHAR(255),

    created_at                  DATETIME NOT NULL,
    modified_at                 DATETIME,

    FOREIGN KEY (oauth2_app_id) REFERENCES oauth2_app (id) ON DELETE CASCADE,
    UNIQUE(oauth2_app_id, identifier) ON CONFLICT ABORT
);
