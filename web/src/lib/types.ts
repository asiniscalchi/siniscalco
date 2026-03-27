import type { AssetsQuery, FxRatesQuery } from "@/gql/types";

export type {
  AccountType,
  AssetType,
  TransactionType,
  SummaryStatus,
  RefreshAvailability,
  FxRateSummaryItem,
  AccountSummary,
  PortfolioAccountTotal,
  PortfolioCashByCurrency,
  PortfolioAllocationSlice,
  PortfolioHolding,
  PortfolioSummary,
} from "@/gql/types";

export type Asset = AssetsQuery["assets"][number];
export type FxRateSummary = FxRatesQuery["fxRates"];
