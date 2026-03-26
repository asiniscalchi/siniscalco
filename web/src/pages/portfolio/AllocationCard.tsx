import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DonutChart, SLICE_COLORS } from "@/components/ui/donut-chart";
import { type PortfolioAllocationSliceResponse } from "@/lib/api";
import { MoneyText } from "@/lib/money";

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
            <DonutChart
              slices={slices.map((s, i) => ({
                value: s.value,
                color: SLICE_COLORS[i % SLICE_COLORS.length],
              }))}
            />
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
