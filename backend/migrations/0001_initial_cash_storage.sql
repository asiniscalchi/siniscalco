CREATE TABLE currencies (
    code TEXT PRIMARY KEY
);

CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('bank', 'broker')),
    base_currency TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (base_currency) REFERENCES currencies(code)
);

CREATE TABLE account_balances (
    account_id INTEGER NOT NULL,
    currency TEXT NOT NULL,
    amount DECIMAL(20,8) NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (account_id, currency),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (currency) REFERENCES currencies(code)
);

CREATE TABLE fx_rates (
    from_currency TEXT NOT NULL,
    to_currency TEXT NOT NULL,
    rate DECIMAL(20,8) NOT NULL CHECK (rate > 0),
    PRIMARY KEY (from_currency, to_currency),
    FOREIGN KEY (from_currency) REFERENCES currencies(code),
    FOREIGN KEY (to_currency) REFERENCES currencies(code)
);

INSERT INTO currencies (code)
VALUES ('EUR'), ('USD'), ('GBP'), ('CHF');
