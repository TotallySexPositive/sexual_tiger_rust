CREATE TABLE IF NOT EXISTS playlist (
    playlist_id	INTEGER,
    name	TEXT NOT NULL UNIQUE,
    num_songs	INTEGER NOT NULL DEFAULT 0,
    created_by	TEXT NOT NULL,
    PRIMARY KEY(playlist_id)
);

CREATE INDEX IF NOT EXISTS playlist_name ON playlist (
    name
);