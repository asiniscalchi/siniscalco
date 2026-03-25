import { Cell, Pie, PieChart, Tooltip } from "recharts";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { type PortfolioAllocationSliceResponse } from "@/lib/api";
import { MoneyText } from "@/lib/money";

const SLICE_COLORS = [
  "#3b82f6", // blue
  "#10b981", // emerald
  "#f59e0b", // amber
  "#ef4444", // red
  "#8b5cf6", // violet
  "#06b6d4", // cyan
  "#f97316", // orange
  "#84cc16", // lime
];

type Slice = PortfolioAllocationSliceResponse & { value: number };

export function AllocationCard({
  allocations,
  isPartial,
  displayCurrency,
  hideValues,
}: {
  allocations: PortfolioAllocationSliceResponse[];
  isPartial: boolean;
  displayCurrency: string;
  hideValues: boolean;
}) {
  if (allocations.length === 0) {
    return (
      <Card className="bg-background">
        <CardHeader className="border-b">
          <CardTitle>Allocation by asset class</CardTitle>
        </CardHeader>
        <CardContent className="pb-6 pt-6">
          <p className="text-sm text-muted-foreground">
            No allocation data available.
          </p>
        </CardContent>
      </Card>
    );
  }

  const slices: Slice[] = allocations
    .map((a) => ({ ...a, value: Number(a.amount) }))
    .sort((a, b) => b.value - a.value);

  const total = slices.reduce((sum, s) => sum + s.value, 0);

  return (
    <Card className="bg-background">
      <CardHeader className="border-b">
        <CardTitle>Allocation by asset class</CardTitle>
      </CardHeader>
      <CardContent className="pb-6 pt-6">
        {isPartial && (
          <p className="mb-4 text-xs font-medium text-destructive">
            Allocation incomplete: some assets could not be valued.
          </p>
        )}
        <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-start">
          <div className="shrink-0">
            <PieChart width={180} height={180}>
              <Pie
                data={slices}
                cx={85}
                cy={85}
                innerRadius={52}
                outerRadius={82}
                dataKey="value"
                strokeWidth={2}
              >
                {slices.map((_, index) => (
                  <Cell
                    key={index}
                    fill={SLICE_COLORS[index % SLICE_COLORS.length]}
                  />
                ))}
              </Pie>
              {!hideValues && (
                <Tooltip
                  formatter={(value) =>
                    value != null
                      ? `${value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${displayCurrency}`
                      : ""
                  }
                />
              )}
            </PieChart>
          </div>
          <div className="w-full space-y-3">
            {slices.map((slice, index) => {
              const percentage =
                total > 0 ? (slice.value / total) * 100 : 0;
              return (
                <div
                  key={slice.label}
                  className="flex items-center justify-between gap-4"
                >
                  <div className="flex items-center gap-2">
                    <span
                      className="inline-block h-3 w-3 shrink-0 rounded-full"
                      style={{
                        backgroundColor:
                          SLICE_COLORS[index % SLICE_COLORS.length],
                      }}
                    />
                    <span className="text-sm font-medium">{slice.label}</span>
                  </div>
                  <div className="flex items-center gap-3 text-right">
                    <MoneyText
                      className="text-sm text-muted-foreground"
                      currency={displayCurrency}
                      hidden={hideValues}
                      value={slice.amount}
                    />
                    <span className="w-14 text-right text-xs text-muted-foreground font-mono tabular-nums">
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
