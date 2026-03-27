import { type PortfolioSummary } from "@/lib/api";
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
          {summary.totalValueStatus === "ok" && summary.totalValueAmount ? (
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
        <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          {summary.totalValueStatus === "conversion_unavailable" && (
            <span className="font-medium text-destructive">
              Conversion data unavailable
            </span>
          )}
          {summary.totalValueStatus === "ok" && (
            <span>Converted to {summary.displayCurrency}</span>
          )}
          <span>•</span>
          <span>
            Last FX update:{" "}
            {summary.fxLastUpdated ? formatTimestamp(summary.fxLastUpdated) : "unavailable"}
          </span>
          {summary.fxRefreshStatus === "unavailable" && (
            <>
              <span>•</span>
              <span className="font-medium text-destructive">
                FX refresh unavailable
              </span>
            </>
          )}
        </div>
        {summary.fxRefreshStatus === "unavailable" && summary.fxRefreshError ? (
          <div className="text-xs text-destructive/90">
            {summary.fxRefreshError}
          </div>
        ) : null}
      </div>
    </section>
  );
}

function formatTimestamp(timestamp: string) {
  const [date, time] = timestamp.split(" ");
  if (!date || !time) {
    return timestamp.slice(0, 16);
  }

  return `${date} ${time.slice(0, 5)}`;
}
