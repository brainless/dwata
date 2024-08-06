-- This table stores updates from different process in the app backend

CREATE TABLE process_log
(
    id                          INTEGER PRIMARY KEY,

    task                        VARCHAR(30) NOT NULL,
    arguments                   JSON,
    status                      VARCHAR(30) NOT NULL,

    is_sent_to_frontend         BOOLEAN NOT NULL,

    created_at                  DATETIME NOT NULL,
    modified_at                 DATETIME
);
