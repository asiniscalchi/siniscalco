export type AccountType = "BANK" | "BROKER" | "CRYPTO";
export type AssetType = "STOCK" | "ETF" | "BOND" | "CRYPTO" | "CASH_EQUIVALENT" | "OTHER";
export type TransactionType = "BUY" | "SELL";
export type SummaryStatus = "OK" | "CONVERSION_UNAVAILABLE";
export type RefreshAvailability = "AVAILABLE" | "UNAVAILABLE";

export type FxRateSummaryItem = {
  currency: string;
  rate: string;
};

export type FxRateSummary = {
  targetCurrency: string;
  rates: FxRateSummaryItem[];
  lastUpdated: string | null;
  refreshStatus: RefreshAvailability;
  refreshError: string | null;
};

export type PortfolioAccountTotal = {
  id: number;
  name: string;
  accountType: AccountType;
  summaryStatus: SummaryStatus;
  cashTotalAmount: string | null;
  assetTotalAmount: string | null;
  totalAmount: string | null;
  totalCurrency: string;
};

export type PortfolioCashByCurrency = {
  currency: string;
  amount: string;
  convertedAmount: string | null;
};

export type PortfolioAllocationSlice = {
  label: string;
  amount: string;
};

export type PortfolioHolding = {
  assetId: number;
  symbol: string;
  name: string;
  value: string;
};

export type PortfolioSummary = {
  displayCurrency: string;
  totalValueStatus: SummaryStatus;
  totalValueAmount: string | null;
  accountTotals: PortfolioAccountTotal[];
  cashByCurrency: PortfolioCashByCurrency[];
  fxLastUpdated: string | null;
  fxRefreshStatus: RefreshAvailability;
  fxRefreshError: string | null;
  allocationTotals: PortfolioAllocationSlice[];
  allocationIsPartial: boolean;
  holdings: PortfolioHolding[];
  holdingsIsPartial: boolean;
};

export type AccountSummary = {
  id: number;
  name: string;
  accountType: AccountType;
  baseCurrency: string;
  summaryStatus: SummaryStatus;
  cashTotalAmount: string | null;
  assetTotalAmount: string | null;
  totalAmount: string | null;
  totalCurrency: string | null;
};

export type Asset = {
  id: number;
  symbol: string;
  name: string;
  assetType: AssetType;
  quoteSymbol: string | null;
  isin: string | null;
  currentPrice: string | null;
  currentPriceCurrency: string | null;
  currentPriceAsOf: string | null;
  totalQuantity: string | null;
};
