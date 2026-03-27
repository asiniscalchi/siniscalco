import { useQuery } from "@apollo/client/react";

import { PORTFOLIO_QUERY, type PortfolioSummary } from "@/lib/api";

import { FxRatesFooter } from "./FxRatesFooter";
import { PortfolioErrorState } from "./PortfolioErrorState";
import { PortfolioLoadingState } from "./PortfolioLoadingState";
import { PortfolioReadyState } from "./PortfolioReadyState";

export function PortfolioPage() {
  const { data, loading, error, refetch } = useQuery<{ portfolio: PortfolioSummary }>(PORTFOLIO_QUERY);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-8">
      {loading ? <PortfolioLoadingState /> : null}
      {!loading && error ? (
        <PortfolioErrorState onRetry={() => void refetch()} />
      ) : null}
      {!loading && !error && data ? (
        <>
          <PortfolioReadyState summary={data.portfolio} />
          <FxRatesFooter />
        </>
      ) : null}
    </div>
  );
}
