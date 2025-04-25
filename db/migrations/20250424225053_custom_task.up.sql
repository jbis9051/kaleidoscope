-- Add up migration script here
CREATE TABLE custom_metadata (
     id INTEGER PRIMARY KEY AUTOINCREMENT,
     media_id INTEGER NOT NULL,
     version INTEGER NOT NULL,
     key TEXT NOT NULL,
     value TEXT NULL,
     created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,
     FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE CASCADE,
     UNIQUE (media_id, version, key)
)