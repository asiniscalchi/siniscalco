import { useQuery } from "@apollo/client/react";
import { MARKET_DATA_POLL_INTERVAL } from "@/lib/apollo";

import { type AssetsQuery } from "@/gql/types";

import { ASSETS_QUERY } from "./assets-query";

type AssetItem = AssetsQuery["assets"][number];

type MoverEntry = {
  asset: AssetItem;
  gainPct: number;
  pct: string;
  positive: boolean;
};

function buildMoverEntry(asset: AssetItem): MoverEntry | null {
  const { currentPrice, previousClose } = asset;
  if (!currentPrice || !previousClose) return null;

  const price = Number(currentPrice);
  const close = Number(previousClose);
  if (Number.isNaN(price) || Number.isNaN(close) || close === 0) return null;

  const gainPct = ((price - close) / close) * 100;
  const sign = gainPct >= 0 ? "+" : "";
  const pct = `${sign}${gainPct.toFixed(2)}%`;

  return { asset, gainPct, pct, positive: gainPct >= 0 };
}

export function TopMoversCard() {
  const { data, loading } = useQuery<AssetsQuery>(ASSETS_QUERY, { fetchPolicy: "cache-and-network", pollInterval: MARKET_DATA_POLL_INTERVAL });
  const assets = data?.assets ?? [];

  if (loading && assets.length === 0) return null;

  const entries = assets.map(buildMoverEntry).filter((e): e is MoverEntry => e !== null);
  if (entries.length === 0) return null;

  const winners = [...entries]
    .filter((e) => e.gainPct > 0)
    .sort((a, b) => b.gainPct - a.gainPct)
    .slice(0, 3);

  const losers = [...entries]
    .filter((e) => e.gainPct < 0)
    .sort((a, b) => a.gainPct - b.gainPct)
    .slice(0, 3);

  if (winners.length === 0 && losers.length === 0) return null;

  return (
    <div className="flex flex-wrap items-center gap-x-6 gap-y-2 px-1 text-sm">
      <div className="flex items-center gap-4" data-testid="top-movers-winners">
        {winners.map((e) => (
          <span key={e.asset.id} className="flex items-center gap-1.5">
            <span className="font-semibold">{e.asset.symbol}</span>
            <span className="font-mono tabular-nums text-xs text-green-600 dark:text-green-400">{e.pct}</span>
          </span>
        ))}
      </div>
      {winners.length > 0 && losers.length > 0 && (
        <div className="h-4 w-px bg-border" />
      )}
      <div className="flex items-center gap-4" data-testid="top-movers-losers">
        {losers.map((e) => (
          <span key={e.asset.id} className="flex items-center gap-1.5">
            <span className="font-semibold">{e.asset.symbol}</span>
            <span className="font-mono tabular-nums text-xs text-red-600 dark:text-red-400">{e.pct}</span>
          </span>
        ))}
      </div>
    </div>
  );
}
