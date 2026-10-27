import type { AssetsQuery, FxRatesQuery, AccountsListQuery, PortfolioQuery } from "@/gql/types";

export type {
  AccountType,
  AssetType,
  TransactionType,
  SummaryStatus,
  RefreshAvailability,
} from "@/gql/types";

export type Asset = AssetsQuery["assets"][number];
export type FxRateSummary = FxRatesQuery["fxRates"];

export type FxRateSummaryItem = FxRateSummary["rates"][number];
export type AccountSummary = AccountsListQuery["accounts"][number];
export type PortfolioSummary = PortfolioQuery["portfolio"];
export type PortfolioAccountTotal = PortfolioSummary["accountTotals"][number];
export type PortfolioCashByCurrency = PortfolioSummary["cashByCurrency"][number];
export type PortfolioAllocationSlice = PortfolioSummary["allocationTotals"][number];
export type PortfolioHolding = PortfolioSummary["holdings"][number];
