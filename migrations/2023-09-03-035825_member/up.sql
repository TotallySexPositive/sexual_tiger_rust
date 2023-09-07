CREATE TABLE IF NOT EXISTS member (
    member_id	INTEGER,
    username	TEXT NOT NULL UNIQUE,
    added_at	TEXT,
    PRIMARY KEY(member_id)
);