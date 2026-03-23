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
    amount TEXT NOT NULL CHECK (
        length(amount) > 0
        AND trim(amount) = amount
        AND amount NOT LIKE '%.%.%'
        AND (instr(amount, '-') = 0 OR instr(amount, '-') = 1)
        AND length(amount) - length(replace(amount, '-', '')) <= 1
        AND replace(replace(amount, '-', ''), '.', '') NOT GLOB '*[^0-9]*'
        AND length(replace(replace(amount, '-', ''), '.', '')) BETWEEN 1 AND 20
        AND (
            (
                instr(replace(amount, '-', ''), '.') = 0
                AND length(replace(amount, '-', '')) BETWEEN 1 AND 12
            )
            OR (
                instr(replace(amount, '-', ''), '.') > 0
                AND length(substr(replace(amount, '-', ''), 1, instr(replace(amount, '-', ''), '.') - 1)) BETWEEN 1 AND 12
                AND length(substr(replace(amount, '-', ''), instr(replace(amount, '-', ''), '.') + 1)) BETWEEN 1 AND 8
            )
        )
    ),
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
    quantity TEXT NOT NULL,
    unit_price TEXT NOT NULL,
    currency_code TEXT NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (asset_id) REFERENCES assets(id),
    FOREIGN KEY (currency_code) REFERENCES currencies(code)
);

CREATE TABLE fx_rates (
    from_currency TEXT NOT NULL,
    to_currency TEXT NOT NULL,
    rate TEXT NOT NULL CHECK (
        length(rate) > 0
        AND trim(rate) = rate
        AND rate NOT LIKE '%.%.%'
        AND rate NOT GLOB '*[^0-9.]*'
        AND length(replace(rate, '.', '')) BETWEEN 1 AND 20
        AND (
            (instr(rate, '.') = 0 AND length(rate) BETWEEN 1 AND 12)
            OR (
                instr(rate, '.') > 0
                AND length(substr(rate, 1, instr(rate, '.') - 1)) BETWEEN 1 AND 12
                AND length(substr(rate, instr(rate, '.') + 1)) BETWEEN 1 AND 8
            )
        )
        AND CAST(rate AS REAL) > 0
    ),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (from_currency, to_currency),
    FOREIGN KEY (from_currency) REFERENCES currencies(code),
    FOREIGN KEY (to_currency) REFERENCES currencies(code)
);

INSERT INTO currencies (code)
VALUES ('EUR'), ('USD'), ('GBP'), ('CHF');
