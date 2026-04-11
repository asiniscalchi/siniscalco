CREATE TABLE asset_quote_sources (
    asset_id INTEGER PRIMARY KEY,
    quote_symbol TEXT NOT NULL CHECK (length(trim(quote_symbol)) > 0),
    provider TEXT NOT NULL CHECK (length(trim(provider)) > 0),
    last_success_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE
);
