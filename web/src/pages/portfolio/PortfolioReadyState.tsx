import { type AssetType } from "@/gql/types";
import { type PortfolioSummary } from "@/lib/types";
import { useUiState } from "@/lib/ui-state";

import { TopMoversCard } from "@/pages/assets/TopMoversCard";
import { AllocationCard } from "./AllocationCard";
import { PortfolioAccountBreakdown } from "./PortfolioAccountBreakdown";
import { PortfolioEmptyState } from "./PortfolioEmptyState";
import { PortfolioHistoryCard } from "./PortfolioHistoryCard";
import { PortfolioSummarySection } from "./PortfolioSummarySection";
import { TopHoldingsCard } from "./TopHoldingsCard";

export function PortfolioReadyState({ summary, assetTypeById }: { summary: PortfolioSummary; assetTypeById: Map<number, AssetType> }) {
  const { hideValues } = useUiState();
  const hasCashData = summary.cashByCurrency.length > 0;
  const currentValue =
    summary.totalValueStatus === "OK" && summary.totalValueAmount
      ? Number(summary.totalValueAmount)
      : undefined;
  const totalGain =
    summary.totalValueStatus === "OK" && summary.totalGainAmount
      ? Number(summary.totalGainAmount)
      : undefined;
  const baseValue =
    currentValue != null && totalGain != null ? currentValue - totalGain : undefined;

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
      <TopMoversCard />
      <PortfolioHistoryCard
        baseValue={baseValue}
        currentValue={currentValue}
      />
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
        <div className="min-w-0 lg:col-span-2">
          <TopHoldingsCard
            holdings={summary.holdings}
            isPartial={summary.holdingsIsPartial}
            displayCurrency={summary.displayCurrency}
            hideValues={hideValues}
            assetTypeById={assetTypeById}
          />
        </div>
      </div>
    </div>
  );
}
