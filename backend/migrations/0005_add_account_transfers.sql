CREATE TABLE account_transfers (
    id INTEGER PRIMARY KEY,
    from_account_id INTEGER NOT NULL,
    to_account_id INTEGER NOT NULL,
    from_currency TEXT NOT NULL,
    from_amount INTEGER NOT NULL CHECK (typeof(from_amount) = 'integer' AND from_amount > 0),
    to_currency TEXT NOT NULL,
    to_amount INTEGER NOT NULL CHECK (typeof(to_amount) = 'integer' AND to_amount > 0),
    transfer_date TEXT NOT NULL CHECK (length(transfer_date) = 10 AND transfer_date GLOB '????-??-??'),
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (from_account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (to_account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (from_currency) REFERENCES currencies(code),
    FOREIGN KEY (to_currency) REFERENCES currencies(code),
    CHECK (from_account_id != to_account_id)
);
