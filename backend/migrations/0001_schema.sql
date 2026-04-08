CREATE TABLE currencies (
    code TEXT PRIMARY KEY
);

CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    account_type TEXT NOT NULL CHECK (account_type IN ('bank', 'broker', 'crypto')),
    base_currency TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (base_currency) REFERENCES currencies(code)
);

CREATE TABLE cash_entries (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    currency TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (typeof(amount) = 'integer'),
    source TEXT NOT NULL CHECK (source IN ('deposit', 'asset_transaction', 'transfer')),
    source_id INTEGER,
    date TEXT NOT NULL DEFAULT (date('now')) CHECK (length(date) = 10 AND date GLOB '????-??-??'),
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (currency) REFERENCES currencies(code),
    CHECK (
        (source = 'deposit' AND source_id IS NULL) OR
        (source IN ('transfer', 'asset_transaction') AND source_id IS NOT NULL)
    )
);

CREATE INDEX cash_entries_account_currency_idx ON cash_entries(account_id, currency);

-- cash_entries are ledger rows: once written they must never be modified or removed.
-- Corrections must be made via compensating rows with the opposite sign.
CREATE TRIGGER cash_entries_immutable_update
BEFORE UPDATE ON cash_entries
BEGIN
    SELECT RAISE(ABORT, 'cash_entries rows are immutable; use a compensating entry to correct');
END;

CREATE TRIGGER cash_entries_immutable_delete
BEFORE DELETE ON cash_entries
BEGIN
    SELECT RAISE(ABORT, 'cash_entries rows are immutable; use a compensating entry to correct');
END;

CREATE TABLE assets (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE CHECK (length(trim(symbol)) > 0),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    asset_type TEXT NOT NULL CHECK (
        asset_type IN ('STOCK', 'ETF', 'BOND', 'CRYPTO', 'CASH_EQUIVALENT', 'OTHER')
    ),
    quote_symbol TEXT,
    isin TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE UNIQUE INDEX assets_isin_unique_idx ON assets(isin) WHERE isin IS NOT NULL;

CREATE UNIQUE INDEX assets_quote_symbol_unique_idx
ON assets(quote_symbol) WHERE quote_symbol IS NOT NULL;

CREATE TABLE asset_transactions (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    asset_id INTEGER NOT NULL,
    transaction_type TEXT NOT NULL CHECK (transaction_type IN ('BUY', 'SELL')),
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

CREATE INDEX asset_transactions_account_trade_date_idx
ON asset_transactions(account_id, trade_date DESC, id DESC);

CREATE INDEX asset_transactions_account_asset_idx
ON asset_transactions(account_id, asset_id);

CREATE TABLE fx_rates (
    from_currency TEXT NOT NULL,
    to_currency TEXT NOT NULL,
    rate INTEGER NOT NULL CHECK (typeof(rate) = 'integer' AND rate > 0),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (from_currency, to_currency),
    FOREIGN KEY (from_currency) REFERENCES currencies(code),
    FOREIGN KEY (to_currency) REFERENCES currencies(code)
);

CREATE TABLE asset_prices (
    asset_id INTEGER PRIMARY KEY,
    price INTEGER NOT NULL CHECK (typeof(price) = 'integer' AND price >= 0),
    currency_code TEXT NOT NULL,
    as_of TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (currency_code) REFERENCES currencies(code)
);

CREATE TABLE asset_price_history (
    id INTEGER PRIMARY KEY,
    asset_id INTEGER NOT NULL,
    price INTEGER NOT NULL CHECK (typeof(price) = 'integer' AND price >= 0),
    currency_code TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (currency_code) REFERENCES currencies(code)
);

CREATE INDEX asset_price_history_asset_recorded ON asset_price_history (asset_id, recorded_at);

CREATE TABLE portfolio_snapshots (
    id INTEGER PRIMARY KEY,
    total_value INTEGER NOT NULL CHECK (typeof(total_value) = 'integer' AND total_value >= 0),
    currency_code TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (currency_code) REFERENCES currencies(code)
);

CREATE UNIQUE INDEX portfolio_snapshots_date_currency
ON portfolio_snapshots (date(recorded_at), currency_code);

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

-- Transfer records are immutable once created. Reversal is done by inserting
-- compensating cash_entries and then deleting the transfer row; updates are never needed.
CREATE TRIGGER account_transfers_immutable_update
BEFORE UPDATE ON account_transfers
BEGIN
    SELECT RAISE(ABORT, 'account_transfers rows are immutable');
END;

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

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO currencies (code)
VALUES ('EUR'), ('USD'), ('GBP'), ('CHF');
