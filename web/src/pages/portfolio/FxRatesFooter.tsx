import { type FxRateSummary } from "@/lib/api";

function formatFxRate(rate: string) {
  const parsedRate = Number(rate);

  if (Number.isNaN(parsedRate)) {
    return rate;
  }

  return parsedRate.toFixed(4);
}

export function FxRatesFooter({ summary }: { summary: FxRateSummary }) {
  if (summary.rates.length === 0) {
    return null;
  }

  return (
    <footer
      className="mt-8 flex flex-wrap items-center justify-between border-t py-4 text-[11px] font-mono text-muted-foreground/60"
      aria-label={`FX rates against ${summary.targetCurrency}`}
    >
      {summary.rates.map((rate) => (
        <div key={rate.currency} className="flex items-center gap-1.5">
          <span className="font-bold">{rate.currency}</span>
          <span>{formatFxRate(rate.rate)}</span>
        </div>
      ))}
      {summary.refreshStatus === "UNAVAILABLE" && (
        <div
          className="text-destructive/80 font-bold uppercase tracking-wider"
          title={summary.refreshError || "FX refresh unavailable"}
        >
          Refresh Failed
        </div>
      )}
    </footer>
  );
}
