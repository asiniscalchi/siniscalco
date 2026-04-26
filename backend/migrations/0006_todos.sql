CREATE TABLE todos (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    due_date TEXT NOT NULL CHECK (length(due_date) = 10 AND due_date GLOB '????-??-??'),
    symbol TEXT,
    completed INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (symbol IS NULL OR length(trim(symbol)) > 0)
);

CREATE INDEX todos_status_due_date_idx ON todos(completed, due_date ASC, id ASC);
