INSERT INTO fx_rate_history (from_currency, to_currency, rate, recorded_at)
SELECT from_currency, to_currency, rate, updated_at
FROM fx_rates
WHERE NOT EXISTS (
    SELECT 1
    FROM fx_rate_history
    WHERE fx_rate_history.from_currency = fx_rates.from_currency
      AND fx_rate_history.to_currency = fx_rates.to_currency
      AND fx_rate_history.recorded_at = fx_rates.updated_at
);
