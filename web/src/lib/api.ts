import { gql } from "@apollo/client/core";
import { CombinedGraphQLErrors } from "@apollo/client/errors";

export function getApiBaseUrl() {
  return import.meta.env.VITE_API_BASE_URL?.trim() || "http://127.0.0.1:3000";
}

export function getHealthApiUrl() {
  return new URL("/health", getApiBaseUrl()).toString();
}

export function extractGqlErrorMessage(error: unknown, fallback: string): string {
  if (CombinedGraphQLErrors.is(error)) {
    return error.errors[0]?.message ?? fallback;
  }
  return fallback;
}

export function extractGqlFieldErrors(
  error: unknown,
): Record<string, string[]> | null {
  if (CombinedGraphQLErrors.is(error)) {
    const extensions = error.errors[0]?.extensions as
      | Record<string, unknown>
      | undefined;
    if (extensions?.field_errors) {
      return extensions.field_errors as Record<string, string[]>;
    }
  }
  return null;
}

// ── Types ──────────────────────────────────────────────────────────────────────

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

export type Balance = {
  currency: string;
  amount: string;
  updatedAt: string;
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

export type AccountDetail = {
  id: number;
  name: string;
  accountType: AccountType;
  baseCurrency: string;
  summaryStatus: SummaryStatus;
  cashTotalAmount: string | null;
  assetTotalAmount: string | null;
  totalAmount: string | null;
  totalCurrency: string | null;
  createdAt: string;
  balances: Balance[];
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
  createdAt?: string;
  updatedAt?: string;
};

export type AssetPosition = {
  accountId: number;
  assetId: number;
  quantity: string;
};

export type Transaction = {
  id: number;
  accountId: number;
  assetId: number;
  transactionType: TransactionType;
  tradeDate: string;
  quantity: string;
  unitPrice: string;
  currencyCode: string;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
};

// ── Queries ────────────────────────────────────────────────────────────────────

export const PORTFOLIO_QUERY = gql`
  {
    portfolio {
      displayCurrency totalValueStatus totalValueAmount
      fxLastUpdated fxRefreshStatus fxRefreshError
      allocationIsPartial holdingsIsPartial
      accountTotals {
        id name accountType summaryStatus
        cashTotalAmount assetTotalAmount totalAmount totalCurrency
      }
      cashByCurrency { currency amount convertedAmount }
      allocationTotals { label amount }
      holdings { assetId symbol name value }
    }
  }
`;

export const FX_RATES_QUERY = gql`
  {
    fxRates {
      targetCurrency lastUpdated refreshStatus refreshError
      rates { currency rate }
    }
  }
`;

export const ACCOUNTS_QUERY = gql`
  {
    accounts {
      id name accountType baseCurrency summaryStatus
      cashTotalAmount assetTotalAmount totalAmount totalCurrency
    }
  }
`;

export const ACCOUNT_QUERY = gql`
  query Account($id: Int!) {
    account(id: $id) {
      id name accountType baseCurrency summaryStatus createdAt
      cashTotalAmount assetTotalAmount totalAmount totalCurrency
      balances { currency amount updatedAt }
    }
  }
`;

export const ACCOUNT_POSITIONS_QUERY = gql`
  query AccountPositions($accountId: Int!) {
    accountPositions(accountId: $accountId) {
      accountId assetId quantity
    }
  }
`;

export const ASSETS_QUERY = gql`
  {
    assets {
      id symbol name assetType quoteSymbol isin
      currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
    }
  }
`;

export const TRANSACTIONS_QUERY = gql`
  query Transactions($accountId: Int) {
    transactions(accountId: $accountId) {
      id accountId assetId transactionType tradeDate
      quantity unitPrice currencyCode notes createdAt updatedAt
    }
  }
`;

export const CURRENCIES_QUERY = gql`
  {
    currencies
  }
`;

// ── Mutations ──────────────────────────────────────────────────────────────────

export const CREATE_ACCOUNT_MUTATION = gql`
  mutation CreateAccount($input: CreateAccountInput!) {
    createAccount(input: $input) {
      id name accountType baseCurrency summaryStatus createdAt
      cashTotalAmount assetTotalAmount totalAmount totalCurrency
      balances { currency amount updatedAt }
    }
  }
`;

export const DELETE_ACCOUNT_MUTATION = gql`
  mutation DeleteAccount($id: Int!) {
    deleteAccount(id: $id)
  }
`;

export const UPSERT_BALANCE_MUTATION = gql`
  mutation UpsertBalance($accountId: Int!, $input: UpsertBalanceInput!) {
    upsertBalance(accountId: $accountId, input: $input) {
      currency amount updatedAt
    }
  }
`;

export const DELETE_BALANCE_MUTATION = gql`
  mutation DeleteBalance($accountId: Int!, $currency: String!) {
    deleteBalance(accountId: $accountId, currency: $currency)
  }
`;

export const CREATE_ASSET_MUTATION = gql`
  mutation CreateAsset($input: CreateAssetInput!) {
    createAsset(input: $input) {
      id symbol name assetType quoteSymbol isin
      currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
      createdAt updatedAt
    }
  }
`;

export const UPDATE_ASSET_MUTATION = gql`
  mutation UpdateAsset($id: Int!, $input: UpdateAssetInput!) {
    updateAsset(id: $id, input: $input) {
      id symbol name assetType quoteSymbol isin
      currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
      createdAt updatedAt
    }
  }
`;

export const DELETE_ASSET_MUTATION = gql`
  mutation DeleteAsset($id: Int!) {
    deleteAsset(id: $id)
  }
`;

export const CREATE_TRANSACTION_MUTATION = gql`
  mutation CreateTransaction($input: CreateTransactionInput!) {
    createTransaction(input: $input) {
      id accountId assetId transactionType tradeDate
      quantity unitPrice currencyCode notes createdAt updatedAt
    }
  }
`;

export const UPDATE_TRANSACTION_MUTATION = gql`
  mutation UpdateTransaction($id: Int!, $input: UpdateTransactionInput!) {
    updateTransaction(id: $id, input: $input) {
      id accountId assetId transactionType tradeDate
      quantity unitPrice currencyCode notes createdAt updatedAt
    }
  }
`;

export const DELETE_TRANSACTION_MUTATION = gql`
  mutation DeleteTransaction($id: Int!) {
    deleteTransaction(id: $id)
  }
`;
