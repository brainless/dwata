-- Add migration script here
CREATE TABLE user_account
(
    id         INTEGER PRIMARY KEY,
    json_data  JSONB,
    created_at DATETIME NOT NULL
);

CREATE TABLE chat_thread
(
    id            INTEGER PRIMARY KEY,
    json_data     JSONB,
    created_by_id INTEGER  NOT NULL,
    created_at    DATETIME NOT NULL,
    FOREIGN KEY (created_by_id) REFERENCES user_account (id)
);

CREATE TABLE chat_reply
(
    id             INTEGER PRIMARY KEY,
    json_data      JSONB,
    chat_thread_id INTEGER  NOT NULL,
    created_by_id  INTEGER  NOT NULL,
    created_at     DATETIME NOT NULL,
    FOREIGN KEY (created_by_id) REFERENCES user_account (id),
    FOREIGN KEY (chat_thread_id) REFERENCES chat_thread (id) ON DELETE CASCADE
);