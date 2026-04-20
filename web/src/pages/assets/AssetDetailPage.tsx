import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";
import { useParams } from "react-router-dom";

import { TradingViewChart } from "@/components/TradingViewChart";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { type AssetQuery } from "@/gql/types";
import { extractGqlErrorMessage } from "@/lib/gql";

import { formatDailyGain, formatGain, formatPrice, formatTotalValue, quoteSourceLabel } from "./asset-utils";

const ASSET_QUERY = gql`
  query Asset($id: Int!) {
    asset(id: $id) {
      id symbol name assetType quoteSymbol isin
      quoteSourceSymbol quoteSourceProvider quoteSourceLastSuccessAt
      currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
      avgCostBasis avgCostBasisCurrency
      previousClose previousCloseCurrency
      convertedTotalValue convertedTotalValueCurrency
    }
  }
`;

export function AssetDetailPage() {
  const { assetId } = useParams<{ assetId: string }>();
  const numericId = assetId ? parseInt(assetId, 10) : 0;

  const { data, loading, error, refetch } = useQuery<AssetQuery>(ASSET_QUERY, {
    variables: { id: numericId },
    skip: !assetId,
  });

  if (loading) {
    return <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />;
  }

  if (error || !data?.asset) {
    return (
      <Card className="border-destructive/30 bg-background">
        <CardHeader>
          <CardTitle>Error</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <p className="text-sm text-muted-foreground">
            {error ? extractGqlErrorMessage(error, "Failed to load asset") : "Asset not found"}
          </p>
          <Button onClick={() => void refetch()}>Retry</Button>
        </CardContent>
      </Card>
    );
  }

  const asset = data.asset;
  const chartSymbol = asset.quoteSourceSymbol ?? asset.quoteSymbol ?? asset.symbol;
  const daily = formatDailyGain(asset);
  const gain = formatGain(asset);
  const totalValue = formatTotalValue(asset);
  const source = quoteSourceLabel(asset);

  return (
    <div className="mx-auto flex min-w-0 w-full max-w-4xl flex-col gap-6">
<Card className="bg-background overflow-hidden">
        <CardHeader className="pb-3">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">{asset.symbol}</h1>
              <p className="text-sm text-muted-foreground">{asset.name}</p>
            </div>
            <span className="inline-flex items-center rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide">
              {asset.assetType.replace("_", " ")}
            </span>
          </div>
        </CardHeader>
        <CardContent className="px-4 pb-4">
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4 text-sm mb-6">
            <div>
              <p className="text-[11px] uppercase tracking-wide text-muted-foreground">Price</p>
              <p className="font-mono tabular-nums font-medium">{formatPrice(asset)}</p>
              {source && <p className="text-[11px] text-muted-foreground">{source}</p>}
            </div>
            {daily && (
              <div>
                <p className="text-[11px] uppercase tracking-wide text-muted-foreground">24h</p>
                <p className={`font-mono tabular-nums ${daily.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"}`}>
                  {daily.abs ? `${daily.abs} (${daily.pct})` : daily.pct}
                </p>
              </div>
            )}
            {totalValue && (
              <div>
                <p className="text-[11px] uppercase tracking-wide text-muted-foreground">Holdings</p>
                <p className="font-mono tabular-nums font-medium">{totalValue}</p>
                {asset.totalQuantity && (
                  <p className="text-[11px] text-muted-foreground font-mono">{asset.totalQuantity} units</p>
                )}
              </div>
            )}
            {gain && (
              <div>
                <p className="text-[11px] uppercase tracking-wide text-muted-foreground">Total gain</p>
                <p className={`font-mono tabular-nums ${gain.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"}`}>
                  {gain.abs ?? gain.pct}
                </p>
                {gain.abs && <p className={`text-[11px] font-mono tabular-nums ${gain.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"}`}>{gain.pct}</p>}
              </div>
            )}
          </div>
        </CardContent>
        <div className="-mx-0 w-full">
          <TradingViewChart symbol={chartSymbol} assetType={asset.assetType} />
        </div>
      </Card>
    </div>
  );
}
