CREATE TABLE IF NOT EXISTS song (
    song_id	INTEGER,
    name	TEXT NOT NULL UNIQUE,
    hash_id	TEXT NOT NULL UNIQUE,
    duration	INTEGER,
    is_clip	INTEGER DEFAULT 0,
    num_plays	INTEGER DEFAULT 0,
    last_played	INTEGER,
    url	TEXT UNIQUE,
    source	TEXT NOT NULL,
    added_by	TEXT,
    PRIMARY KEY(song_id)
);

CREATE INDEX IF NOT EXISTS song_name ON song (
    name
);