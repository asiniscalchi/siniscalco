-- OPENING has no cash impact, but its cost basis needs FX into account base.
-- Use the direct pair (transaction currency -> account base), as BUY/SELL do.
-- Prefer the latest observation on/before the trade day (including that day's
-- final observation). If none exists, use the earliest later observation.
-- Ties use history id, matching select_historical_fx_rate in storage/fx.rs.
-- With no history for the direct pair, preserve the stored rate; do not invent
-- an inverse, multi-hop, or live rate. Same-currency positions always use 1.
-- Cash ledger, BUY/SELL rates, and transaction timestamps remain untouched.
UPDATE asset_transactions
SET fx_rate = CASE
    WHEN currency_code = (SELECT base_currency FROM accounts WHERE id = asset_transactions.account_id)
    THEN 1000000
    ELSE COALESCE(
        (SELECT rate FROM fx_rate_history
         WHERE from_currency = asset_transactions.currency_code
           AND to_currency = (SELECT base_currency FROM accounts WHERE id = asset_transactions.account_id)
           AND date(recorded_at) <= asset_transactions.trade_date
         ORDER BY recorded_at DESC, id DESC LIMIT 1),
        (SELECT rate FROM fx_rate_history
         WHERE from_currency = asset_transactions.currency_code
           AND to_currency = (SELECT base_currency FROM accounts WHERE id = asset_transactions.account_id)
           AND date(recorded_at) > asset_transactions.trade_date
         ORDER BY recorded_at ASC, id ASC LIMIT 1),
        fx_rate
    )
END
WHERE transaction_type = 'OPENING';
