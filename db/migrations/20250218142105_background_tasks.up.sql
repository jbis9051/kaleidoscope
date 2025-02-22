-- Add up migration script here
INSERT INTO kv (key, value, created_at, updated_at)
VALUES ('migration_version', '0', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

CREATE TABLE queue
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id   INT     NOT NULL,
    task       TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (media_id) REFERENCES media (id) ON DELETE CASCADE
);