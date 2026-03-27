import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { type PortfolioSummary } from "@/lib/types";
import { MoneyText } from "@/lib/money";

import { PortfolioProgressItem } from "./PortfolioProgressItem";

export function PortfolioCurrencyBreakdown({
  balances,
  hideValues,
  totalValue,
}: {
  balances: PortfolioSummary["cashByCurrency"];
  hideValues: boolean;
  totalValue: number | null;
}) {
  return (
    <Card className="self-start bg-background">
      <CardHeader className="border-b">
        <CardTitle>Cash By Currency</CardTitle>
      </CardHeader>
      <CardContent className="pb-6 pt-6">
        <div className="space-y-6">
          {balances.map((balance) => {
            const balanceValue = balance.convertedAmount
              ? Number(balance.convertedAmount)
              : null;
            const percentage =
              totalValue && balanceValue ? (balanceValue / totalValue) * 100 : 0;

            return (
              <PortfolioProgressItem
                key={balance.currency}
                label={balance.currency}
                percentage={percentage}
                value={
                  <MoneyText
                    className="text-right text-xs text-muted-foreground"
                    currency={balance.currency}
                    hidden={hideValues}
                    maximumFractionDigits={8}
                    minimumFractionDigits={0}
                    value={balance.amount}
                  />
                }
              />
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
