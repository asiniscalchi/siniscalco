CREATE TABLE chat_threads (
    id TEXT PRIMARY KEY,
    title TEXT,
    status TEXT NOT NULL DEFAULT 'regular' CHECK (status IN ('regular', 'archived')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE chat_messages (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES chat_threads(id) ON DELETE CASCADE,
    parent_id TEXT,
    content_json TEXT NOT NULL,
    run_config_json TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX chat_messages_thread_id_idx ON chat_messages(thread_id);
