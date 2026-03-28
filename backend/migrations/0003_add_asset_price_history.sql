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
