CREATE TABLE fx_rate_history (
    id INTEGER PRIMARY KEY,
    from_currency TEXT NOT NULL,
    to_currency TEXT NOT NULL,
    rate INTEGER NOT NULL CHECK (typeof(rate) = 'integer' AND rate > 0),
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (from_currency) REFERENCES currencies(code),
    FOREIGN KEY (to_currency) REFERENCES currencies(code)
);

CREATE INDEX fx_rate_history_pair_recorded
ON fx_rate_history (from_currency, to_currency, recorded_at);
