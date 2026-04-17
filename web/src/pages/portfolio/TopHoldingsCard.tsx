import { ItemLabel } from "@/components/ItemLabel";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DonutChart, SLICE_COLORS } from "@/components/ui/donut-chart";
import { CASH_SLICE_COLORS } from "@/lib/colors";
import { MoneyText } from "@/lib/money";
import { type PortfolioHolding } from "@/lib/types";

type ChartItem = {
  name: string;
  fullName: string;
  value: number;
  assetId?: number | null;
  color: string;
};

function assignColors(
  items: Omit<ChartItem, "color">[],
): ChartItem[] {
  let cashIdx = 0;
  let colorIdx = 0;
  return items.map((item) => ({
    ...item,
    color: item.assetId === null
      ? CASH_SLICE_COLORS[cashIdx++ % CASH_SLICE_COLORS.length]
      : SLICE_COLORS[colorIdx++ % SLICE_COLORS.length],
  }));
}

export function TopHoldingsCard({
  holdings,
  isPartial,
  displayCurrency,
  hideValues,
}: {
  holdings: PortfolioHolding[];
  isPartial: boolean;
  displayCurrency: string;
  hideValues: boolean;
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

  const top5 = chartableHoldings.slice(0, 5);
  const others = chartableHoldings.slice(5);

  const chartData = assignColors([
    ...top5.map((h) => ({
      name: h.symbol,
      fullName: h.name,
      value: h.value,
      assetId: h.assetId,
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
          },
        ]
      : []),
  ]);

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
                    <MoneyText
                      className="text-right text-xs text-muted-foreground"
                      currency={displayCurrency}
                      hidden={hideValues}
                      value={item.value.toString()}
                    />
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
