import { type PortfolioSummaryResponse } from "@/lib/api";
import { useUiState } from "@/lib/ui-state";

import { PortfolioAccountBreakdown } from "./PortfolioAccountBreakdown";
import { PortfolioCurrencyBreakdown } from "./PortfolioCurrencyBreakdown";
import { PortfolioEmptyState } from "./PortfolioEmptyState";
import { PortfolioPageHeader } from "./PortfolioPageHeader";
import { PortfolioSummarySection } from "./PortfolioSummarySection";

type PortfolioSummary = PortfolioSummaryResponse;

export function PortfolioReadyState({ summary }: { summary: PortfolioSummary }) {
  const { hideValues } = useUiState();
  const hasCashData = summary.cash_by_currency.length > 0;
  const totalValue = summary.total_value_amount
    ? Number(summary.total_value_amount)
    : null;

  if (!hasCashData) {
    return (
      <div className="flex flex-col gap-4">
        <PortfolioPageHeader />
        <PortfolioEmptyState />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-8">
      <PortfolioPageHeader />
      <PortfolioSummarySection summary={summary} hideValues={hideValues} />
      <div className="grid gap-8 lg:grid-cols-[1fr_350px]">
        <PortfolioAccountBreakdown
          accountTotals={summary.account_totals}
          hideValues={hideValues}
          totalValue={totalValue}
        />
        <PortfolioCurrencyBreakdown
          balances={summary.cash_by_currency}
          hideValues={hideValues}
          totalValue={totalValue}
        />
      </div>
    </div>
  );
}
