import { useQuery } from "@apollo/client/react";
import { MARKET_DATA_POLL_INTERVAL } from "@/lib/apollo";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatMoney } from "@/lib/format-money";
import { type AssetsQuery } from "@/gql/types";

import { ASSETS_QUERY } from "./assets-query";

type AssetItem = AssetsQuery["assets"][number];

type MoverEntry = {
  asset: AssetItem;
  gainPct: number;
  pct: string;
  abs: string | null;
  positive: boolean;
};

function buildMoverEntry(asset: AssetItem): MoverEntry | null {
  const { currentPrice, currentPriceCurrency, previousClose, previousCloseCurrency } = asset;
  if (!currentPrice || !previousClose) return null;

  const price = Number(currentPrice);
  const close = Number(previousClose);
  if (Number.isNaN(price) || Number.isNaN(close) || close === 0) return null;

  const gainPct = ((price - close) / close) * 100;
  const sign = gainPct >= 0 ? "+" : "";
  const pct = `${sign}${gainPct.toFixed(2)}%`;
  const sameCurrency = currentPriceCurrency && previousCloseCurrency === currentPriceCurrency;
  const gainAbs = sameCurrency ? price - close : null;
  const abs =
    gainAbs !== null
      ? `${sign}${formatMoney(gainAbs, currentPriceCurrency ?? undefined, false).text}`
      : null;

  return { asset, gainPct, pct, abs, positive: gainPct >= 0 };
}

function MoverRow({ entry }: { entry: MoverEntry }) {
  const color = entry.positive
    ? "text-green-600 dark:text-green-400"
    : "text-red-600 dark:text-red-400";
  return (
    <div className="flex items-center justify-between gap-2 py-1.5">
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-semibold leading-tight">{entry.asset.symbol}</div>
        <div className="truncate text-[10px] text-muted-foreground">{entry.asset.name}</div>
      </div>
      <div className={`shrink-0 text-right font-mono tabular-nums text-sm ${color}`}>
        <div>{entry.pct}</div>
        {entry.abs && <div className="text-[10px]">{entry.abs}</div>}
      </div>
    </div>
  );
}

function MoverColumn({
  title,
  entries,
  emptyText,
  testId,
}: {
  title: string;
  entries: MoverEntry[];
  emptyText: string;
  testId: string;
}) {
  return (
    <div className="min-w-0 flex-1" data-testid={testId}>
      <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
        {title}
      </h3>
      {entries.length === 0 ? (
        <p className="text-sm text-muted-foreground">{emptyText}</p>
      ) : (
        <div className="divide-y">
          {entries.map((entry) => (
            <MoverRow key={entry.asset.id} entry={entry} />
          ))}
        </div>
      )}
    </div>
  );
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
    <Card className="bg-background">
      <CardHeader>
        <CardTitle>Top Movers</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-6 sm:flex-row sm:gap-8">
          <MoverColumn
            title="Winners"
            entries={winners}
            emptyText="No gainers today"
            testId="top-movers-winners"
          />
          <div className="hidden w-px bg-border sm:block" />
          <MoverColumn
            title="Losers"
            entries={losers}
            emptyText="No losers today"
            testId="top-movers-losers"
          />
        </div>
      </CardContent>
    </Card>
  );
}
