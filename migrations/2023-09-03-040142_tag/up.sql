CREATE TABLE IF NOT EXISTS image_tag (
    image_tag_id	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    image_id	INTEGER NOT NULL,
    tag_id	INTEGER NOT NULL,
    FOREIGN KEY(tag_id) REFERENCES tag(tag_id) ON DELETE CASCADE,
    FOREIGN KEY(image_id) REFERENCES image(image_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tag (
    tag_id	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    name	TEXT NOT NULL UNIQUE,
    description	TEXT
);

CREATE INDEX IF NOT EXISTS tag_id ON tag (
    tag_id
);