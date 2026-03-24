import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { type PortfolioSummaryResponse } from "@/lib/api";
import { MoneyText } from "@/lib/money";

import { PortfolioProgressItem } from "./PortfolioProgressItem";

type PortfolioSummary = PortfolioSummaryResponse;

export function PortfolioAccountBreakdown({
  accountTotals,
  hideValues,
  totalValue,
}: {
  accountTotals: PortfolioSummary["account_totals"];
  hideValues: boolean;
  totalValue: number | null;
}) {
  const sortedAccountTotals = [...accountTotals].sort((a, b) => {
    const valueA = a.total_amount ? Number(a.total_amount) : 0;
    const valueB = b.total_amount ? Number(b.total_amount) : 0;
    return valueB - valueA;
  });

  return (
    <Card className="self-start bg-background">
      <CardHeader className="border-b">
        <CardTitle>Cash By Account</CardTitle>
      </CardHeader>
      <CardContent className="pt-6">
        <div className="space-y-6">
          {sortedAccountTotals.map((account) => {
            const accountValue = account.total_amount ? Number(account.total_amount) : 0;
            const percentage =
              totalValue && account.summary_status === "ok"
                ? (accountValue / totalValue) * 100
                : 0;

            return (
              <PortfolioProgressItem
                key={account.id}
                label={account.name}
                meta={account.account_type}
                percentage={percentage}
                value={
                  account.summary_status === "ok" && account.total_amount ? (
                    <MoneyText
                      className="text-right text-xs text-muted-foreground"
                      currency={account.total_currency}
                      hidden={hideValues}
                      value={account.total_amount}
                    />
                  ) : (
                    <span className="text-xs font-medium text-muted-foreground">
                      Conversion unavailable
                    </span>
                  )
                }
              />
            );
          })}
        </div>
      </CardContent>
      {accountTotals.some((account) => account.summary_status !== "ok") ? (
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
