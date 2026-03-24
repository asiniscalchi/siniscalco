type ApiErrorResponse = {
  error?: string;
  message: string;
};

export type CurrencyResponse = {
  code: string;
};

export type FxRateSummaryItemResponse = {
  currency: string;
  rate: string;
};

export type FxRateSummaryResponse = {
  target_currency: string;
  rates: FxRateSummaryItemResponse[];
  last_updated: string | null;
  refresh_status: "available" | "unavailable";
  refresh_error: string | null;
};

export type PortfolioAccountTotalResponse = {
  id: number;
  name: string;
  account_type: string;
  summary_status: "ok" | "conversion_unavailable";
  total_amount: string | null;
  total_currency: string;
};

export type PortfolioCashByCurrencyResponse = {
  currency: string;
  amount: string;
  converted_amount: string | null;
};

export type PortfolioSummaryResponse = {
  display_currency: string;
  total_value_status: "ok" | "conversion_unavailable";
  total_value_amount: string | null;
  account_totals: PortfolioAccountTotalResponse[];
  cash_by_currency: PortfolioCashByCurrencyResponse[];
  fx_last_updated: string | null;
  fx_refresh_status: "available" | "unavailable";
  fx_refresh_error: string | null;
};

export type AssetResponse = {
  id: number;
  symbol: string;
  name: string;
  asset_type: string;
  quote_symbol: string | null;
  isin: string | null;
  current_price: string | null;
  current_price_currency: string | null;
  current_price_as_of: string | null;
};

export type AssetPositionResponse = {
  account_id: number;
  asset_id: number;
  quantity: string;
};

export type CreatedAssetResponse = {
  id: number;
  symbol: string;
  name: string;
  asset_type: string;
  quote_symbol: string | null;
  isin: string | null;
  current_price: string | null;
  current_price_currency: string | null;
  current_price_as_of: string | null;
  created_at: string;
  updated_at: string;
};

export function getApiBaseUrl() {
  return import.meta.env.VITE_API_BASE_URL?.trim() || "http://127.0.0.1:3000";
}

export function getHealthApiUrl() {
  return new URL("/health", getApiBaseUrl()).toString();
}

export function getAccountsApiUrl() {
  return new URL("/accounts", getApiBaseUrl()).toString();
}

export function getCurrenciesApiUrl() {
  return new URL("/currencies", getApiBaseUrl()).toString();
}

export function getFxRatesApiUrl() {
  return new URL("/fx-rates", getApiBaseUrl()).toString();
}

export function getPortfolioApiUrl() {
  return new URL("/portfolio", getApiBaseUrl()).toString();
}

export function getAccountDetailApiUrl(accountId: string) {
  return new URL(`/accounts/${accountId}`, getApiBaseUrl()).toString();
}

export function getAccountBalanceApiUrl(accountId: string, currency: string) {
  return new URL(
    `/accounts/${accountId}/balances/${currency}`,
    getApiBaseUrl(),
  ).toString();
}

export function getAccountPositionsApiUrl(accountId: string) {
  return new URL(`/accounts/${accountId}/positions`, getApiBaseUrl()).toString();
}

export function getAssetsApiUrl() {
  return new URL("/assets", getApiBaseUrl()).toString();
}

export function getAssetDetailApiUrl(assetId: string | number) {
  return new URL(`/assets/${assetId}`, getApiBaseUrl()).toString();
}

export function getTransactionsApiUrl(accountId?: string) {
  const url = new URL("/transactions", getApiBaseUrl());
  if (accountId) {
    url.searchParams.append("account_id", accountId);
  }
  return url.toString();
}

export function getTransactionDetailApiUrl(transactionId: string | number) {
  return new URL(`/transactions/${transactionId}`, getApiBaseUrl()).toString();
}

export async function readApiErrorMessage(
  response: Response,
  fallbackMessage: string,
) {
  try {
    const data = (await response.json()) as ApiErrorResponse;
    return data.message || fallbackMessage;
  } catch {
    return fallbackMessage;
  }
}
