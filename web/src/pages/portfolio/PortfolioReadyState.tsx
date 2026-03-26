import { type PortfolioSummaryResponse } from "@/lib/api";
import { useUiState } from "@/lib/ui-state";

import { AllocationCard } from "./AllocationCard";
import { PortfolioAccountBreakdown } from "./PortfolioAccountBreakdown";
import { PortfolioEmptyState } from "./PortfolioEmptyState";
import { PortfolioSummarySection } from "./PortfolioSummarySection";
import { TopHoldingsCard } from "./TopHoldingsCard";

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
      <PortfolioSummarySection summary={summary} hideValues={hideValues} />
      <div className="grid gap-8 lg:grid-cols-2">
        <PortfolioAccountBreakdown
          accountTotals={summary.account_totals}
          hideValues={hideValues}
        />
        <AllocationCard
          allocations={summary.allocation_totals}
          isPartial={summary.allocation_is_partial}
          displayCurrency={summary.display_currency}
          hideValues={hideValues}
        />
        <TopHoldingsCard
          holdings={summary.holdings}
          isPartial={summary.holdings_is_partial}
          displayCurrency={summary.display_currency}
          hideValues={hideValues}
          totalValue={totalValue}
        />
      </div>
    </div>
  );
}
