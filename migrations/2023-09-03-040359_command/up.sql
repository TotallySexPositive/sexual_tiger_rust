CREATE TABLE IF NOT EXISTS command (
    command	TEXT NOT NULL UNIQUE,
    default_access INTEGER NOT NULL,
    PRIMARY KEY(command)
);