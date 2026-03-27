import { type PortfolioSummary } from "@/lib/api";
import { useUiState } from "@/lib/ui-state";

import { AllocationCard } from "./AllocationCard";
import { PortfolioAccountBreakdown } from "./PortfolioAccountBreakdown";
import { PortfolioEmptyState } from "./PortfolioEmptyState";
import { PortfolioSummarySection } from "./PortfolioSummarySection";
import { TopHoldingsCard } from "./TopHoldingsCard";

export function PortfolioReadyState({ summary }: { summary: PortfolioSummary }) {
  const { hideValues } = useUiState();
  const hasCashData = summary.cashByCurrency.length > 0;
  const totalValue = summary.totalValueAmount
    ? Number(summary.totalValueAmount)
    : null;

  if (!hasCashData) {
    return (
      <div className="flex flex-col gap-4">
        <PortfolioEmptyState />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-8">
      <PortfolioSummarySection summary={summary} hideValues={hideValues} />
      <div className="grid gap-8 lg:grid-cols-2">
        <PortfolioAccountBreakdown
          accountTotals={summary.accountTotals}
          hideValues={hideValues}
        />
        <AllocationCard
          allocations={summary.allocationTotals}
          isPartial={summary.allocationIsPartial}
          displayCurrency={summary.displayCurrency}
          hideValues={hideValues}
        />
        <TopHoldingsCard
          holdings={summary.holdings}
          isPartial={summary.holdingsIsPartial}
          displayCurrency={summary.displayCurrency}
          hideValues={hideValues}
          totalValue={totalValue}
        />
      </div>
    </div>
  );
}
