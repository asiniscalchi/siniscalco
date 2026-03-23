CREATE TABLE currencies (
    code TEXT PRIMARY KEY
);

CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    account_type TEXT NOT NULL CHECK (account_type IN ('bank', 'broker')),
    base_currency TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (base_currency) REFERENCES currencies(code)
);

CREATE TABLE account_balances (
    account_id INTEGER NOT NULL,
    currency TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (typeof(amount) = 'integer'),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (account_id, currency),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (currency) REFERENCES currencies(code)
);

CREATE TABLE assets (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE CHECK (length(trim(symbol)) > 0),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    asset_type TEXT NOT NULL CHECK (
        asset_type IN ('STOCK', 'ETF', 'BOND', 'CRYPTO', 'CASH_EQUIVALENT', 'OTHER')
    ),
    isin TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE UNIQUE INDEX assets_isin_unique_idx ON assets(isin) WHERE isin IS NOT NULL;

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

INSERT INTO currencies (code)
VALUES ('EUR'), ('USD'), ('GBP'), ('CHF');
