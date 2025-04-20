-- Add up migration script here
CREATE TABLE job (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    media_uuid TEXT NOT NULL,
    task_name TEXT NOT NULL,
    status TEXT NOT NULL,
    estimated_completion INTEGER DEFAULT NULL,
    success_data TEXT DEFAULT NULL,
    failure_data TEXT DEFAULT NULL,
    created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP
);