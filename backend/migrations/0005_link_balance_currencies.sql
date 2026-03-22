PRAGMA foreign_keys = OFF;

CREATE TABLE account_balances_new (
    account_id INTEGER NOT NULL,
    currency TEXT NOT NULL,
    amount DECIMAL(20,8) NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (account_id, currency),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (currency) REFERENCES currencies(code)
);

INSERT INTO account_balances_new (account_id, currency, amount, updated_at)
SELECT account_id, currency, amount, updated_at
FROM account_balances;

DROP TABLE account_balances;
ALTER TABLE account_balances_new RENAME TO account_balances;

PRAGMA foreign_key_check;
PRAGMA foreign_keys = ON;
