-- Extend asset_transactions to support OPENING transaction type.
-- SQLite does not support ALTER TABLE to modify CHECK constraints, so we
-- recreate the table with the updated constraint.
-- Note: sqlx wraps this migration in a transaction, so no explicit BEGIN/COMMIT
-- or PRAGMA foreign_keys needed here.

CREATE TABLE asset_transactions_new (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    asset_id INTEGER NOT NULL,
    transaction_type TEXT NOT NULL CHECK (transaction_type IN ('BUY', 'SELL', 'OPENING')),
    trade_date TEXT NOT NULL CHECK (
        length(trade_date) = 10
        AND trade_date GLOB '????-??-??'
    ),
    quantity INTEGER NOT NULL CHECK (typeof(quantity) = 'integer' AND quantity > 0),
    unit_price INTEGER NOT NULL CHECK (typeof(unit_price) = 'integer' AND unit_price >= 0),
    currency_code TEXT NOT NULL,
    fx_rate INTEGER NOT NULL DEFAULT 1000000,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (asset_id) REFERENCES assets(id),
    FOREIGN KEY (currency_code) REFERENCES currencies(code)
);

INSERT INTO asset_transactions_new SELECT * FROM asset_transactions;

DROP TABLE asset_transactions;

ALTER TABLE asset_transactions_new RENAME TO asset_transactions;

CREATE INDEX asset_transactions_account_trade_date_idx
ON asset_transactions(account_id, trade_date DESC, id DESC);

CREATE INDEX asset_transactions_account_asset_idx
ON asset_transactions(account_id, asset_id);
