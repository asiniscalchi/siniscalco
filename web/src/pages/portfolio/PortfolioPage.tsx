import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";
import { MARKET_DATA_POLL_INTERVAL } from "@/lib/apollo";

import { type PortfolioQuery } from "@/gql/types";

const PORTFOLIO_QUERY = gql`
  query Portfolio {
    portfolio {
      displayCurrency totalValueStatus totalValueAmount
      gain24hAmount totalGainAmount
      fxLastUpdated fxRefreshStatus fxRefreshError
      allocationIsPartial holdingsIsPartial
      accountTotals {
        id name accountType summaryStatus
        cashTotalAmount assetTotalAmount totalAmount totalCurrency
      }
      cashByCurrency { currency amount convertedAmount }
      allocationTotals { label amount }
      holdings { assetId symbol name value }
    }
  }
`;

import { FxRatesFooter } from "./FxRatesFooter";
import { PortfolioErrorState } from "./PortfolioErrorState";
import { PortfolioLoadingState } from "./PortfolioLoadingState";
import { PortfolioReadyState } from "./PortfolioReadyState";

export function PortfolioPage() {
  const { data, loading, error, refetch } = useQuery<PortfolioQuery>(PORTFOLIO_QUERY, { fetchPolicy: "cache-and-network", pollInterval: MARKET_DATA_POLL_INTERVAL });

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
