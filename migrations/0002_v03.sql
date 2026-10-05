ALTER TABLE items ADD COLUMN alive INTEGER;
ALTER TABLE items ADD COLUMN fail_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN checked_at TEXT;
ALTER TABLE items ADD COLUMN copies INTEGER NOT NULL DEFAULT 0;

CREATE TABLE users(
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE sessions(
    token      TEXT PRIMARY KEY,
    user_id    INTEGER NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE ip_blacklist(
    ip         TEXT PRIMARY KEY,
    note       TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE stats_daily(
    day       TEXT NOT NULL,
    item_id   INTEGER NOT NULL,
    downloads INTEGER NOT NULL DEFAULT 0,
    copies    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(day, item_id)
);

CREATE TABLE stats_source(
    day    TEXT NOT NULL,
    source TEXT NOT NULL,
    count  INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(day, source)
);
