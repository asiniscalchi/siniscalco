PRAGMA foreign_keys = OFF;

CREATE TABLE accounts_new (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('bank', 'broker')),
    base_currency TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (base_currency) REFERENCES currencies(code)
);

INSERT INTO accounts_new (id, name, account_type, base_currency, created_at)
SELECT id, name, account_type, base_currency, created_at
FROM accounts;

DROP TABLE accounts;
ALTER TABLE accounts_new RENAME TO accounts;

PRAGMA foreign_key_check;
PRAGMA foreign_keys = ON;
