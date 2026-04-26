CREATE TABLE todos_new (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    completed INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO todos_new (id, title, completed, created_at, updated_at)
SELECT id, title, completed, created_at, updated_at
FROM todos;

DROP TABLE todos;

ALTER TABLE todos_new RENAME TO todos;

CREATE INDEX todos_status_idx ON todos(completed, id ASC);
