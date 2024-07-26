-- SQLITE
CREATE TABLE media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    width INT NOT NULL,
    height INT NOT NULL,
    size INT NOT NULL,
    path TEXT NOT NULL,
    liked BOOLEAN NOT NULL,
    is_photo BOOLEAN NOT NULL,
    added_at INTEGER NOT NULL,
    duration INTEGER NULl,
    hash TEXT NOT NULL,
    UNIQUE (hash)
);


CREATE TABLE album (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE album_media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    album_id INT NOT NULL,
    media_id INT NOT NULL,
    FOREIGN KEY (album_id) REFERENCES album (id) ON DELETE CASCADE,
    FOREIGN KEY (media_id) REFERENCES media (id) ON DELETE CASCADE
    UNIQUE (album_id, media_id)
);
