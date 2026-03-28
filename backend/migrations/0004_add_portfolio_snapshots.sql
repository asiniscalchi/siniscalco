CREATE TABLE portfolio_snapshots (
    id INTEGER PRIMARY KEY,
    total_value INTEGER NOT NULL CHECK (typeof(total_value) = 'integer' AND total_value >= 0),
    currency_code TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (currency_code) REFERENCES currencies(code)
);

CREATE UNIQUE INDEX portfolio_snapshots_date_currency
ON portfolio_snapshots (date(recorded_at), currency_code);
