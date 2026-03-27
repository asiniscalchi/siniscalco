import { useQuery } from "@apollo/client/react";

import {
  FX_RATES_QUERY,
  PORTFOLIO_QUERY,
  type FxRateSummary,
  type PortfolioSummary,
} from "@/lib/api";

import { FxRatesFooter } from "./FxRatesFooter";
import { PortfolioErrorState } from "./PortfolioErrorState";
import { PortfolioLoadingState } from "./PortfolioLoadingState";
import { PortfolioReadyState } from "./PortfolioReadyState";

export function PortfolioPage() {
  const { data: portfolioData, loading: portfolioLoading, error: portfolioError, refetch: refetchPortfolio } = useQuery<{ portfolio: PortfolioSummary }>(PORTFOLIO_QUERY);
  const { data: fxData, loading: fxLoading, error: fxError, refetch: refetchFxRates } = useQuery<{ fxRates: FxRateSummary }>(FX_RATES_QUERY);

  const loading = portfolioLoading || fxLoading;
  const error = portfolioError ?? fxError;

  function handleRetry() {
    void refetchPortfolio();
    void refetchFxRates();
  }

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-8">
      {loading ? <PortfolioLoadingState /> : null}
      {!loading && error ? (
        <PortfolioErrorState onRetry={handleRetry} />
      ) : null}
      {!loading && !error && portfolioData && fxData ? (
        <>
          <PortfolioReadyState summary={portfolioData.portfolio} />
          <FxRatesFooter summary={fxData.fxRates} />
        </>
      ) : null}
    </div>
  );
}
