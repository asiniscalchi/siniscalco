import { ItemLabel } from "@/components/ItemLabel";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { DonutChart, SLICE_COLORS } from "@/components/ui/donut-chart";
import { type PortfolioSummary } from "@/lib/types";
import { formatMoney } from "@/lib/format-money";

export function PortfolioAccountBreakdown({
  accountTotals,
  hideValues,
}: {
  accountTotals: PortfolioSummary["accountTotals"];
  hideValues: boolean;
}) {
  const sortedAccountTotals = [...accountTotals].sort((a, b) => {
    const valueA = a.totalAmount ? Number(a.totalAmount) : 0;
    const valueB = b.totalAmount ? Number(b.totalAmount) : 0;
    return valueB - valueA;
  });

  const validAccounts = sortedAccountTotals.filter(
    (a) => a.summaryStatus === "OK" && a.totalAmount,
  );

  const accountsWithIssues = sortedAccountTotals.filter(
    (a) => a.summaryStatus !== "OK" || !a.totalAmount,
  );

  if (validAccounts.length === 0 && accountsWithIssues.length === 0) {
    return (
      <Card className="self-start bg-background">
        <CardHeader className="border-b">
          <CardTitle>Cash By Account</CardTitle>
        </CardHeader>
        <CardContent className="pb-6 pt-6">
          <p className="text-sm text-muted-foreground">
            No account data available.
          </p>
        </CardContent>
      </Card>
    );
  }

  const chartData = validAccounts.map((account) => ({
    name: account.name,
    value: Number(account.totalAmount),
    currency: account.totalCurrency,
  }));

  const total = chartData.reduce((sum, a) => sum + a.value, 0);
  const displayCurrency = validAccounts[0]?.totalCurrency ?? "USD";

  return (
    <Card className="self-start bg-background">
      <CardHeader className="border-b">
        <CardTitle>Cash By Account</CardTitle>
      </CardHeader>
      <CardContent className="pb-6 pt-6">
        {validAccounts.length > 0 ? (
          <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-start">
            <div className="shrink-0">
              <DonutChart
                slices={chartData.map((a, i) => ({
                  value: a.value,
                  color: SLICE_COLORS[i % SLICE_COLORS.length],
                }))}
              />
            </div>
            <div className="w-full space-y-3">
              {chartData.map((account, index) => {
                const percentage = total > 0 ? (account.value / total) * 100 : 0;
                return (
                  <div
                    key={account.name}
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
                      <ItemLabel
                        primary={account.name}
                        secondary={formatMoney(account.value.toString(), displayCurrency, hideValues).text}
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
        ) : (
          <div className="space-y-3">
            {accountsWithIssues.map((account) => (
              <div
                key={account.id}
                className="flex items-center justify-between gap-4"
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">{account.name}</span>
                </div>
                <span className="text-xs font-medium text-muted-foreground">
                  Conversion unavailable
                </span>
              </div>
            ))}
          </div>
        )}
      </CardContent>
      {accountsWithIssues.length > 0 ? (
        <CardFooter className="pt-0">
          <p className="text-xs italic text-muted-foreground">
            * Some accounts are hidden or incomplete due to missing conversion
            rates.
          </p>
        </CardFooter>
      ) : null}
    </Card>
  );
}
