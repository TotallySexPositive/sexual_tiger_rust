CREATE TABLE IF NOT EXISTS access (
    user_id	TEXT NOT NULL,
    command	TEXT NOT NULL,
    is_allowed	INTEGER NOT NULL,
    set_by	TEXT,
    added_at    TEXT,
    PRIMARY KEY(user_id, command),
    FOREIGN KEY(command) REFERENCES command(command) ON DELETE CASCADE
);