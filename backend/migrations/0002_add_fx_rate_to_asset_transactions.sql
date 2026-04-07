-- Store the FX rate used at trade execution time so that cash-impact
-- reversals (on delete or update) always use the original rate instead
-- of the live rate from fx_rates. Defaults to 1.000000 (scaled ×10^6)
-- which is correct for same-currency transactions and serves as a safe
-- fallback for any rows created before this migration.
ALTER TABLE asset_transactions
    ADD COLUMN fx_rate INTEGER NOT NULL DEFAULT 1000000;
