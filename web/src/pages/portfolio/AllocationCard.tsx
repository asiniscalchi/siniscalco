import { ItemLabel } from "@/components/ItemLabel";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DonutChart, SLICE_COLORS } from "@/components/ui/donut-chart";
import { CASH_SLICE_COLORS } from "@/lib/colors";
import { formatMoney } from "@/lib/format-money";
import { type PortfolioAllocationSlice } from "@/lib/types";

type Slice = PortfolioAllocationSlice & { value: number; color: string };

function assignColors(slices: (PortfolioAllocationSlice & { value: number })[]): Slice[] {
  let cashIdx = 0;
  let colorIdx = 0;
  return slices.map((s) => ({
    ...s,
    color: s.label === "Cash"
      ? CASH_SLICE_COLORS[cashIdx++ % CASH_SLICE_COLORS.length]
      : SLICE_COLORS[colorIdx++ % SLICE_COLORS.length],
  }));
}

export function AllocationCard({
  allocations,
  isPartial,
  displayCurrency,
  hideValues,
}: {
  allocations: PortfolioAllocationSlice[];
  isPartial: boolean;
  displayCurrency: string;
  hideValues: boolean;
}) {
  if (allocations.length === 0) {
    return (
      <Card className="bg-background">
        <CardHeader className="border-b">
          <CardTitle>Allocation</CardTitle>
        </CardHeader>
        <CardContent className="pb-6 pt-6">
          <p className="text-sm text-muted-foreground">
            No allocation data available.
          </p>
        </CardContent>
      </Card>
    );
  }

  const slices = assignColors(
    allocations
      .map((a) => ({ ...a, value: Number(a.amount) }))
      .sort((a, b) => b.value - a.value),
  );

  const total = slices.reduce((sum, s) => sum + s.value, 0);

  return (
    <Card className="self-start bg-background">
      <CardHeader className="border-b">
        <CardTitle>Allocation</CardTitle>
      </CardHeader>
      <CardContent className="pb-6 pt-6">
        {isPartial && (
          <p className="mb-4 text-xs font-medium text-destructive">
            Allocation incomplete: some assets could not be valued.
          </p>
        )}
        <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-center">
          <div className="shrink-0">
            <DonutChart
              slices={slices.map((s) => ({ value: s.value, color: s.color }))}
            />
          </div>
          <div className="w-full space-y-3">
            {slices.map((slice) => {
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
                      style={{ backgroundColor: slice.color }}
                    />
                    <ItemLabel
                      primary={slice.label}
                      secondary={formatMoney(slice.amount, displayCurrency, hideValues).text}
                    />
                  </div>
                  <span className="w-14 text-right text-xs text-muted-foreground font-mono tabular-nums">
                    {hideValues ? "•••%" : `${percentage.toFixed(1)}%`}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
