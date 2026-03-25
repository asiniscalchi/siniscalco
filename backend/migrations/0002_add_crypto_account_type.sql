-- SQLite does not support ALTER TABLE ... DROP CONSTRAINT, so we recreate the
-- accounts table to extend the account_type CHECK constraint with 'crypto'.

PRAGMA foreign_keys = OFF;

CREATE TABLE accounts_new (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    account_type TEXT NOT NULL CHECK (account_type IN ('bank', 'broker', 'crypto')),
    base_currency TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (base_currency) REFERENCES currencies(code)
);

INSERT INTO accounts_new SELECT * FROM accounts;

DROP TABLE accounts;

ALTER TABLE accounts_new RENAME TO accounts;

PRAGMA foreign_keys = ON;
