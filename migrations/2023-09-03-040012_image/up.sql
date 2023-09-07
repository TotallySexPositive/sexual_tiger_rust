CREATE TABLE IF NOT EXISTS image (
    image_id	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    hash_id	TEXT NOT NULL UNIQUE,
    extension	TEXT NOT NULL,
    added_by	TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS image_hash_id ON image (
    hash_id
);

CREATE INDEX IF NOT EXISTS image_id ON image (
    image_id
);