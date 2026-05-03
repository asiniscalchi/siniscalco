import { clsx } from "clsx";
import { ItemLabel } from "@/components/ItemLabel";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DonutChart } from "@/components/ui/donut-chart";
import { type AssetType } from "@/gql/types";
import { BOND_COLORS, CASH_SLICE_COLORS, CRYPTO_COLORS, ETF_COLORS, OTHER_COLORS, STOCK_COLORS } from "@/lib/colors";
import { MoneyText } from "@/lib/money";
import { type PortfolioHolding } from "@/lib/types";

type ChartItem = {
  name: string;
  fullName: string;
  value: number;
  assetId?: number | null;
  color: string;
  gain24hAmount?: string | null;
};

const TYPE_PALETTES: Record<AssetType, string[]> = {
  STOCK: STOCK_COLORS,
  ETF: ETF_COLORS,
  CRYPTO: CRYPTO_COLORS,
  BOND: BOND_COLORS,
  CASH_EQUIVALENT: CASH_SLICE_COLORS,
  OTHER: OTHER_COLORS,
};

function assignColors(
  items: Omit<ChartItem, "color">[],
  assetTypeById: Map<number, AssetType>,
): ChartItem[] {
  const typeIndexes = new Map<string, number>();
  return items.map((item) => {
    if (item.assetId === null) {
      const idx = typeIndexes.get("CASH") ?? 0;
      typeIndexes.set("CASH", idx + 1);
      return { ...item, color: CASH_SLICE_COLORS[idx % CASH_SLICE_COLORS.length] };
    }
    const type = item.assetId != null ? (assetTypeById.get(item.assetId) ?? "OTHER") : "OTHER";
    const palette = TYPE_PALETTES[type];
    const idx = typeIndexes.get(type) ?? 0;
    typeIndexes.set(type, idx + 1);
    return { ...item, color: palette[idx % palette.length] };
  });
}

export function TopHoldingsCard({
  holdings,
  isPartial,
  displayCurrency,
  hideValues,
  assetTypeById,
}: {
  holdings: PortfolioHolding[];
  isPartial: boolean;
  displayCurrency: string;
  hideValues: boolean;
  assetTypeById: Map<number, AssetType>;
}) {
  const holdingsList = holdings ?? [];

  if (holdingsList.length === 0 && !isPartial) {
    return (
      <Card className="bg-background">
        <CardHeader className="border-b">
          <CardTitle>Top Holdings</CardTitle>
        </CardHeader>
        <CardContent className="pb-6 pt-6">
          <p className="text-sm text-muted-foreground">
            No holdings data available.
          </p>
        </CardContent>
      </Card>
    );
  }

  const chartableHoldings = holdingsList
    .map((h) => ({ ...h, value: Number(h.value) }))
    .filter((h) => !Number.isNaN(h.value) && h.value > 0);

  if (chartableHoldings.length === 0) {
    return (
      <Card className="bg-background">
        <CardHeader className="border-b">
          <CardTitle>Top Holdings</CardTitle>
        </CardHeader>
        <CardContent className="pb-6 pt-6">
          <p className="text-sm text-muted-foreground">
            No holdings data available.
          </p>
        </CardContent>
      </Card>
    );
  }

  const total = chartableHoldings.reduce((sum, h) => sum + h.value, 0);
  const n = chartableHoldings.length;

  let splitIdx = n;
  if (n > 6) {
    for (let i = 0; i < n; i++) {
      const tailSum = chartableHoldings.slice(i).reduce((s, h) => s + h.value, 0);
      if (total > 0 && tailSum / total <= 0.1) {
        splitIdx = i;
        break;
      }
    }
  }

  const top5 = chartableHoldings.slice(0, splitIdx);
  const others = chartableHoldings.slice(splitIdx);

  const chartData = assignColors([
    ...top5.map((h) => ({
      name: h.symbol,
      fullName: h.name,
      value: h.value,
      assetId: h.assetId,
      gain24hAmount: h.gain24hAmount,
    })),
    ...(others.length > 0
      ? [
          {
            name: "Other",
            fullName: `${others.length} other holding${
              others.length > 1 ? "s" : ""
            }`,
            value: others.reduce((sum, h) => sum + h.value, 0),
            assetId: undefined,
            gain24hAmount: others.some(h => h.gain24hAmount != null)
              ? others.reduce((sum, h) => sum + Number(h.gain24hAmount ?? 0), 0).toString()
              : null,
          },
        ]
      : []),
  ], assetTypeById).map((item) =>
    item.name === "Other" ? { ...item, color: "#f5f5f0" } : item
  );

  const holdingsTotal = chartData.reduce((sum, item) => sum + item.value, 0);

  return (
    <Card className="self-start bg-background">
      <CardHeader className="border-b">
        <CardTitle>Top Holdings</CardTitle>
      </CardHeader>
      <CardContent className="pt-6">
        {isPartial && (
          <p className="mb-4 text-xs font-medium text-destructive">
            Top holdings incomplete: some assets could not be valued.
          </p>
        )}
        <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-center">
          <div
            aria-label="Top holdings donut chart"
            className="shrink-0"
            role="img"
          >
            <DonutChart
              slices={chartData.map((item) => ({
                value: item.value,
                color: item.color,
              }))}
            />
          </div>
          <div className="min-w-0 w-full space-y-3">
            {chartData.map((item) => {
              const percentage =
                holdingsTotal > 0 ? (item.value / holdingsTotal) * 100 : 0;

              return (
                <div
                  key={item.name}
                  className="flex items-center justify-between gap-4"
                >
                  <div className="flex min-w-0 items-center gap-2">
                    <span
                      className="inline-block h-3 w-3 shrink-0 rounded-full"
                      style={{ backgroundColor: item.color }}
                    />
                    <ItemLabel primary={item.name} secondary={item.fullName} />
                  </div>
                  <div className="flex shrink-0 items-center gap-3">
                    <div className="flex flex-col items-end">
                      <MoneyText
                        className="text-right text-xs font-medium"
                        currency={displayCurrency}
                        hidden={hideValues}
                        value={item.value.toString()}
                      />
                      {item.gain24hAmount && !hideValues && (
                        <MoneyText
                          className={clsx(
                            "text-[10px] tabular-nums",
                            Number(item.gain24hAmount) >= 0 ? "text-green-600" : "text-red-600"
                          )}
                          currency={displayCurrency}
                          hidden={hideValues}
                          signDisplay="always"
                          value={item.gain24hAmount}
                        />
                      )}
                    </div>
                    <span className="w-14 text-right font-mono text-xs tabular-nums text-muted-foreground">
                      {hideValues ? "•••%" : `${percentage.toFixed(1)}%`}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
