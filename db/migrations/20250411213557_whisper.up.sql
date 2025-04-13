-- Add up migration script here
CREATE TABLE media_extra (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id INT NOT NULL,
    whisper_version INT NOT NULL DEFAULT -1,
    whisper_language TEXT DEFAULT NULL,
    whisper_confidence REAL DEFAULT NULL,
    whisper_transcript TEXT DEFAULT NULL,
    FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE CASCADE,
    UNIQUE (media_id) ON CONFLICT ABORT
)