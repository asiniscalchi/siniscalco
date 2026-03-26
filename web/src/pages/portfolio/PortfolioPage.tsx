import { useEffect, useState } from "react";

import {
  getFxRatesApiUrl,
  getPortfolioApiUrl,
  type FxRateSummaryResponse,
  type PortfolioSummaryResponse,
} from "@/lib/api";

import { FxRatesFooter } from "./FxRatesFooter";
import { PortfolioErrorState } from "./PortfolioErrorState";
import { PortfolioLoadingState } from "./PortfolioLoadingState";
import { PortfolioReadyState } from "./PortfolioReadyState";

type PortfolioSummary = PortfolioSummaryResponse;

export function PortfolioPage() {
  const [requestState, setRequestState] = useState<
    | { status: "loading" }
    | { status: "error" }
    | { status: "ready"; summary: PortfolioSummary; fxRates: FxRateSummaryResponse }
  >({ status: "loading" });
  const [retryToken, setRetryToken] = useState(0);

  useEffect(() => {
    let cancelled = false;

    async function loadPortfolio() {
      setRequestState({ status: "loading" });

      try {
        const [portfolioResponse, fxRatesResponse] = await Promise.all([
          fetch(getPortfolioApiUrl()),
          fetch(getFxRatesApiUrl()),
        ]);

        if (!portfolioResponse.ok) {
          throw new Error(
            `portfolio request failed with status ${portfolioResponse.status}`,
          );
        }

        if (!fxRatesResponse.ok) {
          throw new Error(
            `fx rates request failed with status ${fxRatesResponse.status}`,
          );
        }

        const [summary, fxRates] = await Promise.all([
          portfolioResponse.json() as Promise<PortfolioSummaryResponse>,
          fxRatesResponse.json() as Promise<FxRateSummaryResponse>,
        ]);

        if (!cancelled) {
          setRequestState({ status: "ready", summary, fxRates });
        }
      } catch {
        if (!cancelled) {
          setRequestState({ status: "error" });
        }
      }
    }

    void loadPortfolio();

    return () => {
      cancelled = true;
    };
  }, [retryToken]);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-8">
      {requestState.status === "loading" ? <PortfolioLoadingState /> : null}
      {requestState.status === "error" ? (
        <PortfolioErrorState onRetry={() => setRetryToken((value) => value + 1)} />
      ) : null}
      {requestState.status === "ready" ? (
        <>
          <PortfolioReadyState summary={requestState.summary} />
          <FxRatesFooter summary={requestState.fxRates} />
        </>
      ) : null}
    </div>
  );
}
