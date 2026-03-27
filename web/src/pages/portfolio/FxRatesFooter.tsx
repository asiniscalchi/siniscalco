import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";

import { type FxRateSummary } from "@/lib/api";

const FX_RATES_QUERY = gql`
  {
    fxRates {
      targetCurrency lastUpdated refreshStatus refreshError
      rates { currency rate }
    }
  }
`;

function formatFxRate(rate: string) {
  const parsedRate = Number(rate);

  if (Number.isNaN(parsedRate)) {
    return rate;
  }

  return parsedRate.toFixed(4);
}

function FxRatesFooterContent({ summary }: { summary: FxRateSummary }) {
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

export function FxRatesFooter() {
  const { data } = useQuery<{ fxRates: FxRateSummary }>(FX_RATES_QUERY);

  if (!data) {
    return null;
  }

  return <FxRatesFooterContent summary={data.fxRates} />;
}
