CREATE TABLE cash_entries (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    currency TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (typeof(amount) = 'integer'),
    source TEXT NOT NULL CHECK (source IN ('deposit', 'asset_transaction', 'transfer')),
    source_id INTEGER,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (currency) REFERENCES currencies(code)
);

CREATE INDEX cash_entries_account_currency_idx ON cash_entries(account_id, currency);

INSERT INTO cash_entries (account_id, currency, amount, source, created_at)
SELECT account_id, currency, amount, 'deposit', updated_at
FROM account_balances
WHERE amount != 0;

DROP TABLE account_balances;
