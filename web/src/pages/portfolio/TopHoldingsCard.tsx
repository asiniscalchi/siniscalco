import { Bar, BarChart, Cell, Tooltip, XAxis, YAxis } from "recharts";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { type PortfolioHoldingResponse } from "@/lib/api";
import { MoneyText } from "@/lib/money";

type Holding = {
  asset_id: number;
  symbol: string;
  name: string;
  value: number;
};

const BAR_COLORS = [
  "#3b82f6",
  "#10b981",
  "#f59e0b",
  "#ef4444",
  "#8b5cf6",
  "#06b6d4",
];

export function TopHoldingsCard({
  holdings,
  isPartial,
  displayCurrency,
  hideValues,
  totalValue,
}: {
  holdings: PortfolioHoldingResponse[];
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
          <CardTitle>Top holdings</CardTitle>
        </CardHeader>
        <CardContent className="pb-6 pt-6">
          <p className="text-sm text-muted-foreground">
            No holdings data available.
          </p>
        </CardContent>
      </Card>
    );
  }

  const chartableHoldings: Holding[] = holdingsList
    .map((h) => ({ ...h, value: Number(h.value) }))
    .filter((h) => !isNaN(h.value) && h.value > 0);

  if (chartableHoldings.length === 0) {
    return (
      <Card className="bg-background">
        <CardHeader className="border-b">
          <CardTitle>Top holdings</CardTitle>
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
    <Card className="bg-background">
      <CardHeader className="border-b">
        <CardTitle>Top holdings</CardTitle>
      </CardHeader>
      <CardContent className="pb-6 pt-6">
        {isPartial && (
          <p className="mb-4 text-xs font-medium text-destructive">
            Top holdings incomplete: some assets could not be valued.
          </p>
        )}
        <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-start">
          <div className="shrink-0">
            <BarChart
              layout="vertical"
              width={200}
              height={Math.max(chartData.length * 32, 70)}
              data={chartData}
              margin={{ top: 0, right: 0, bottom: 0, left: 0 }}
            >
              <XAxis type="number" hide />
              <YAxis type="category" dataKey="name" hide />
              <Bar dataKey="value" radius={[0, 4, 4, 0]} maxBarSize={24}>
                {chartData.map((_, index) => (
                  <Cell
                    key={`cell-${index}`}
                    fill={BAR_COLORS[index % BAR_COLORS.length]}
                  />
                ))}
              </Bar>
              {!hideValues && (
                <Tooltip
                  formatter={(value) => {
                    const num = typeof value === "number" ? value : 0;
                    return `${num.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${displayCurrency}`;
                  }}
                  labelFormatter={(label) => {
                    const item = chartData.find((d) => d.name === label);
                    return item?.fullName || String(label);
                  }}
                />
              )}
            </BarChart>
          </div>
          <div className="w-full space-y-3">
            {chartData.map((item, index) => (
              <div
                key={item.name}
                className="flex items-center justify-between gap-4"
              >
                <div className="flex items-center gap-2">
                  <span
                    className="inline-block h-3 w-3 shrink-0 rounded-full"
                    style={{
                      backgroundColor: BAR_COLORS[index % BAR_COLORS.length],
                    }}
                  />
                  <span className="text-sm font-medium truncate max-w-[120px]">
                    {item.name}
                  </span>
                </div>
                <div className="flex items-center gap-3 text-right">
                  <MoneyText
                    className="text-sm text-muted-foreground"
                    currency={displayCurrency}
                    hidden={hideValues}
                    value={item.value.toString()}
                  />
                  <span className="w-14 text-right text-xs text-muted-foreground font-mono tabular-nums">
                    {hideValues
                      ? "•••%"
                      : `${item.percentage.toFixed(1)}%`}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}