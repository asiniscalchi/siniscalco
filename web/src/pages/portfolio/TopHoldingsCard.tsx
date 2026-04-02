import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { type PortfolioHolding } from "@/lib/types";
import { MoneyText } from "@/lib/money";

import { PortfolioProgressItem } from "./PortfolioProgressItem";

export function TopHoldingsCard({
  holdings,
  isPartial,
  displayCurrency,
  hideValues,
  totalValue,
}: {
  holdings: PortfolioHolding[];
  isPartial: boolean;
  displayCurrency: string;
  hideValues: boolean;
  totalValue: number | null;
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

  type ChartableHolding = {
  assetId?: number | null;
  symbol: string;
  name: string;
  value: number;
};

const chartableHoldings: ChartableHolding[] = holdingsList
  .map((h) => ({ ...h, value: Number(h.value) }))
  .filter((h) => !isNaN(h.value) && h.value > 0);

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

  const chartData = [
    ...top5.map((h) => ({
      name: h.symbol,
      fullName: h.name,
      value: h.value,
      percentage:
        totalValue != null && totalValue > 0 ? (h.value / totalValue) * 100 : 0,
    })),
    ...(others.length > 0
      ? [
          {
            name: "Other",
            fullName: `${others.length} other holding${others.length > 1 ? "s" : ""}`,
            value: others.reduce((sum, h) => sum + h.value, 0),
            percentage:
              totalValue != null && totalValue > 0
                ? (others.reduce((sum, h) => sum + h.value, 0) / totalValue) * 100
                : 0,
          },
        ]
      : []),
  ];

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
        <div className="space-y-6">
          {chartData.map((item) => (
            <PortfolioProgressItem
              key={item.name}
              label={item.name}
              meta={item.fullName}
              percentage={item.percentage}
              value={
                <MoneyText
                  className="text-right text-xs text-muted-foreground"
                  currency={displayCurrency}
                  hidden={hideValues}
                  value={item.value.toString()}
                />
              }
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
