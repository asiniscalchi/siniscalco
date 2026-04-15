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
            amount={summary.dailyGainAmount}
            currency={summary.displayCurrency}
            hideValues={hideValues}
            label="Daily gain"
            testId="portfolio-daily-gain"
          />
          <GainMetric
            amount={summary.totalGainAmount}
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
  currency,
  hideValues,
  label,
  testId,
}: {
  amount: string | null | undefined;
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
        </span>
      ) : (
        <span className="font-medium text-muted-foreground">Unavailable</span>
      )}
    </span>
  );
}
