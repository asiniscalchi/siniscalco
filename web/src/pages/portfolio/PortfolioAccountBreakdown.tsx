import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { DonutChart, SLICE_COLORS } from "@/components/ui/donut-chart";
import { type PortfolioSummaryResponse } from "@/lib/api";
import { MoneyText } from "@/lib/money";

export function PortfolioAccountBreakdown({
  accountTotals,
  hideValues,
}: {
  accountTotals: PortfolioSummaryResponse["account_totals"];
  hideValues: boolean;
}) {
  const sortedAccountTotals = [...accountTotals].sort((a, b) => {
    const valueA = a.total_amount ? Number(a.total_amount) : 0;
    const valueB = b.total_amount ? Number(b.total_amount) : 0;
    return valueB - valueA;
  });

  const validAccounts = sortedAccountTotals.filter(
    (a) => a.summary_status === "ok" && a.total_amount,
  );

  const accountsWithIssues = sortedAccountTotals.filter(
    (a) => a.summary_status !== "ok" || !a.total_amount,
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
    value: Number(account.total_amount),
    currency: account.total_currency,
  }));

  const total = chartData.reduce((sum, a) => sum + a.value, 0);
  const displayCurrency = validAccounts[0]?.total_currency ?? "USD";

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
                      <span className="text-sm font-medium">{account.name}</span>
                    </div>
                    <div className="flex items-center gap-3 text-right">
                      <MoneyText
                        className="text-sm text-muted-foreground"
                        currency={displayCurrency}
                        hidden={hideValues}
                        value={account.value.toString()}
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
