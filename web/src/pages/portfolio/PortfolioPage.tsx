import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";
import { useMemo } from "react";
import { MARKET_DATA_POLL_INTERVAL } from "@/lib/apollo";
import { type AssetsQuery, type AssetType, type PortfolioQuery } from "@/gql/types";
import { ASSETS_QUERY } from "@/pages/assets/assets-query";

import { FxRatesFooter } from "./FxRatesFooter";
import { PortfolioErrorState } from "./PortfolioErrorState";
import { PortfolioLoadingState } from "./PortfolioLoadingState";
import { PortfolioReadyState } from "./PortfolioReadyState";

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

export function PortfolioPage() {
  const { data, loading, error, refetch } = useQuery<PortfolioQuery>(PORTFOLIO_QUERY, { fetchPolicy: "cache-and-network", pollInterval: MARKET_DATA_POLL_INTERVAL });
  const { data: assetsData } = useQuery<AssetsQuery>(ASSETS_QUERY, { fetchPolicy: "cache-only" });

  const assetTypeById = useMemo<Map<number, AssetType>>(() => {
    const map = new Map<number, AssetType>();
    for (const asset of assetsData?.assets ?? []) {
      map.set(asset.id, asset.assetType);
    }
    return map;
  }, [assetsData]);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-8">
      {loading && !data ? <PortfolioLoadingState /> : null}
      {!loading && error && !data ? (
        <PortfolioErrorState onRetry={() => void refetch()} />
      ) : null}
      {data ? (
        <>
          <PortfolioReadyState summary={data.portfolio} assetTypeById={assetTypeById} />
          <FxRatesFooter />
        </>
      ) : null}
    </div>
  );
}
