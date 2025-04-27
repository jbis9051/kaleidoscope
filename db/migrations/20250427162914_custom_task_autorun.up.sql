-- Add up migration script here
ALTER TABLE custom_metadata ADD COLUMN include_search INTEGER NOT NULL DEFAULT FALSE;

CREATE TABLE custom_task_media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id INTEGER NOT NULL,
    task_name TEXT NOT NULL,
    version INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE CASCADE,
    UNIQUE (media_id, version, task_name)
);