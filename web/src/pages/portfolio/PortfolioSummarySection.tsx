import { type PortfolioSummary } from "@/lib/types";
import { MoneyText } from "@/lib/money";

export function PortfolioSummarySection({
  summary,
  hideValues,
}: {
  summary: PortfolioSummary;
  hideValues: boolean;
}) {
  return (
    <section className="space-y-4">
      <div className="flex flex-col items-end gap-1.5 px-1">
        <div className="flex flex-col items-baseline gap-1 sm:flex-row sm:gap-4">
          {summary.totalValueStatus === "OK" && summary.totalValueAmount ? (
            <MoneyText
              className="text-3xl font-bold tracking-tight sm:text-4xl"
              currency={summary.displayCurrency}
              hidden={hideValues}
              value={summary.totalValueAmount}
            />
          ) : (
            <span className="text-xl font-semibold text-muted-foreground sm:text-2xl">
              Conversion unavailable
            </span>
          )}
        </div>
        <div className="flex flex-wrap items-center justify-end gap-3 text-xs text-muted-foreground">
          {summary.totalValueStatus === "CONVERSION_UNAVAILABLE" && (
            <span className="font-medium text-destructive">
              Conversion data unavailable
            </span>
          )}
          <GainMetric
            amount={summary.gain24hAmount}
            totalValue={summary.totalValueAmount}
            currency={summary.displayCurrency}
            hideValues={hideValues}
            label="24h gain"
            testId="portfolio-daily-gain"
          />
          <GainMetric
            amount={summary.totalGainAmount}
            totalValue={summary.totalValueAmount}
            currency={summary.displayCurrency}
            hideValues={hideValues}
            label="Total gain"
            testId="portfolio-total-gain"
          />
          {summary.fxRefreshStatus === "UNAVAILABLE" && (
            <>
              <span>•</span>
              <span className="font-medium text-destructive">
                FX refresh unavailable
              </span>
            </>
          )}
        </div>
        {summary.fxRefreshStatus === "UNAVAILABLE" && summary.fxRefreshError ? (
          <div className="text-xs text-destructive/90">
            {summary.fxRefreshError}
          </div>
        ) : null}
      </div>
    </section>
  );
}

function GainMetric({
  amount,
  totalValue,
  currency,
  hideValues,
  label,
  testId,
}: {
  amount: string | null | undefined;
  totalValue: string | null | undefined;
  currency: string;
  hideValues: boolean;
  label: string;
  testId: string;
}) {
  const numericAmount = amount ? Number(amount) : null;
  const toneClass =
    hideValues || numericAmount === null || numericAmount === 0
      ? "text-muted-foreground"
      : numericAmount > 0
        ? "text-green-600 dark:text-green-400"
        : "text-red-600 dark:text-red-400";
  const sign = !hideValues && numericAmount !== null && numericAmount > 0 ? "+" : "";

  const percentage = (() => {
    if (!amount || !totalValue) return null;
    const gain = Number(amount);
    const basis = Number(totalValue) - gain;
    if (basis === 0) return null;
    return (gain / basis) * 100;
  })();

  const percentageText =
    !hideValues && percentage !== null && numericAmount !== 0
      ? `${percentage >= 0 ? "+" : ""}${percentage.toFixed(2)}%`
      : null;

  return (
    <span className="inline-flex items-center gap-1.5" data-testid={testId}>
      <span>{label}:</span>
      {amount ? (
        <span className={toneClass}>
          {sign}
          <MoneyText
            className="text-sm font-semibold"
            currency={currency}
            hidden={hideValues}
            value={amount}
          />
          {percentageText ? (
            <span className="ml-1 font-semibold">{percentageText}</span>
          ) : null}
        </span>
      ) : (
        <span className="font-medium text-muted-foreground">Unavailable</span>
      )}
    </span>
  );
}
