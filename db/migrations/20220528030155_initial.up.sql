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
    added_at INTEGER NOT NULL
);


CREATE TABLE albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE album_media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    album_id INT NOT NULL,
    photo_id INT NOT NULL,
    FOREIGN KEY (album_id) REFERENCES albums (id) ON DELETE CASCADE,
    FOREIGN KEY (photo_id) REFERENCES media (id) ON DELETE CASCADE
);
