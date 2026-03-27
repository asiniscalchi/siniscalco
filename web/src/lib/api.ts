import { ClientError, GraphQLClient, gql } from "graphql-request";

export function getApiBaseUrl() {
  return import.meta.env.VITE_API_BASE_URL?.trim() || "http://127.0.0.1:3000";
}

export function getHealthApiUrl() {
  return new URL("/health", getApiBaseUrl()).toString();
}

function client() {
  return new GraphQLClient(new URL("/graphql", getApiBaseUrl()).toString());
}

export function extractGqlErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof ClientError) {
    return error.response.errors?.[0]?.message ?? fallback;
  }
  return fallback;
}

export function extractGqlFieldErrors(
  error: unknown,
): Record<string, string[]> | null {
  if (error instanceof ClientError) {
    const extensions = error.response.errors?.[0]?.extensions as
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

export async function fetchPortfolio(): Promise<PortfolioSummary> {
  const query = gql`
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
  const data = await client().request<{ portfolio: PortfolioSummary }>(query);
  return data.portfolio;
}

export async function fetchFxRates(): Promise<FxRateSummary> {
  const query = gql`
    {
      fxRates {
        targetCurrency lastUpdated refreshStatus refreshError
        rates { currency rate }
      }
    }
  `;
  const data = await client().request<{ fxRates: FxRateSummary }>(query);
  return data.fxRates;
}

export async function fetchAccounts(): Promise<AccountSummary[]> {
  const query = gql`
    {
      accounts {
        id name accountType baseCurrency summaryStatus
        cashTotalAmount assetTotalAmount totalAmount totalCurrency
      }
    }
  `;
  const data = await client().request<{ accounts: AccountSummary[] }>(query);
  return data.accounts;
}

export async function fetchAccount(id: number): Promise<AccountDetail> {
  const query = gql`
    query Account($id: Int!) {
      account(id: $id) {
        id name accountType baseCurrency summaryStatus createdAt
        cashTotalAmount assetTotalAmount totalAmount totalCurrency
        balances { currency amount updatedAt }
      }
    }
  `;
  const data = await client().request<{ account: AccountDetail }>(query, {
    id,
  });
  return data.account;
}

export async function fetchAccountPositions(
  accountId: number,
): Promise<AssetPosition[]> {
  const query = gql`
    query AccountPositions($accountId: Int!) {
      accountPositions(accountId: $accountId) {
        accountId assetId quantity
      }
    }
  `;
  const data = await client().request<{ accountPositions: AssetPosition[] }>(
    query,
    { accountId },
  );
  return data.accountPositions;
}

export async function fetchAssets(): Promise<Asset[]> {
  const query = gql`
    {
      assets {
        id symbol name assetType quoteSymbol isin
        currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
      }
    }
  `;
  const data = await client().request<{ assets: Asset[] }>(query);
  return data.assets;
}

export async function fetchTransactions(accountId?: number): Promise<Transaction[]> {
  const query = gql`
    query Transactions($accountId: Int) {
      transactions(accountId: $accountId) {
        id accountId assetId transactionType tradeDate
        quantity unitPrice currencyCode notes createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ transactions: Transaction[] }>(query, {
    accountId: accountId ?? null,
  });
  return data.transactions;
}

export async function fetchCurrencies(): Promise<string[]> {
  const query = gql`
    {
      currencies
    }
  `;
  const data = await client().request<{ currencies: string[] }>(query);
  return data.currencies;
}

// ── Mutations ──────────────────────────────────────────────────────────────────

export async function createAccount(
  name: string,
  accountType: AccountType,
  baseCurrency: string,
): Promise<AccountDetail> {
  const mutation = gql`
    mutation CreateAccount($input: CreateAccountInput!) {
      createAccount(input: $input) {
        id name accountType baseCurrency summaryStatus createdAt
        cashTotalAmount assetTotalAmount totalAmount totalCurrency
        balances { currency amount updatedAt }
      }
    }
  `;
  const data = await client().request<{ createAccount: AccountDetail }>(
    mutation,
    { input: { name, accountType, baseCurrency } },
  );
  return data.createAccount;
}

export async function updateAccount(
  id: number,
  name: string,
  accountType: AccountType,
  baseCurrency: string,
): Promise<AccountDetail> {
  const mutation = gql`
    mutation UpdateAccount($id: Int!, $input: UpdateAccountInput!) {
      updateAccount(id: $id, input: $input) {
        id name accountType baseCurrency summaryStatus createdAt
        cashTotalAmount assetTotalAmount totalAmount totalCurrency
        balances { currency amount updatedAt }
      }
    }
  `;
  const data = await client().request<{ updateAccount: AccountDetail }>(
    mutation,
    { id, input: { name, accountType, baseCurrency } },
  );
  return data.updateAccount;
}

export async function deleteAccount(id: number): Promise<void> {
  const mutation = gql`
    mutation DeleteAccount($id: Int!) {
      deleteAccount(id: $id)
    }
  `;
  await client().request(mutation, { id });
}

export async function upsertBalance(
  accountId: number,
  currency: string,
  amount: string,
): Promise<Balance> {
  const mutation = gql`
    mutation UpsertBalance($accountId: Int!, $input: UpsertBalanceInput!) {
      upsertBalance(accountId: $accountId, input: $input) {
        currency amount updatedAt
      }
    }
  `;
  const data = await client().request<{ upsertBalance: Balance }>(mutation, {
    accountId,
    input: { currency, amount },
  });
  return data.upsertBalance;
}

export async function deleteBalance(
  accountId: number,
  currency: string,
): Promise<void> {
  const mutation = gql`
    mutation DeleteBalance($accountId: Int!, $currency: String!) {
      deleteBalance(accountId: $accountId, currency: $currency)
    }
  `;
  await client().request(mutation, { accountId, currency });
}

export async function createAsset(
  symbol: string,
  name: string,
  assetType: AssetType,
  quoteSymbol?: string | null,
  isin?: string | null,
): Promise<Asset> {
  const mutation = gql`
    mutation CreateAsset($input: CreateAssetInput!) {
      createAsset(input: $input) {
        id symbol name assetType quoteSymbol isin
        currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
        createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ createAsset: Asset }>(mutation, {
    input: {
      symbol,
      name,
      assetType,
      quoteSymbol: quoteSymbol ?? null,
      isin: isin ?? null,
    },
  });
  return data.createAsset;
}

export async function updateAsset(
  id: number,
  symbol: string,
  name: string,
  assetType: AssetType,
  quoteSymbol?: string | null,
  isin?: string | null,
): Promise<Asset> {
  const mutation = gql`
    mutation UpdateAsset($id: Int!, $input: UpdateAssetInput!) {
      updateAsset(id: $id, input: $input) {
        id symbol name assetType quoteSymbol isin
        currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
        createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ updateAsset: Asset }>(mutation, {
    id,
    input: {
      symbol,
      name,
      assetType,
      quoteSymbol: quoteSymbol ?? null,
      isin: isin ?? null,
    },
  });
  return data.updateAsset;
}

export async function deleteAsset(id: number): Promise<void> {
  const mutation = gql`
    mutation DeleteAsset($id: Int!) {
      deleteAsset(id: $id)
    }
  `;
  await client().request(mutation, { id });
}

export async function createTransaction(
  accountId: number,
  assetId: number,
  transactionType: TransactionType,
  tradeDate: string,
  quantity: string,
  unitPrice: string,
  currencyCode: string,
  notes: string | null,
): Promise<Transaction> {
  const mutation = gql`
    mutation CreateTransaction($input: CreateTransactionInput!) {
      createTransaction(input: $input) {
        id accountId assetId transactionType tradeDate
        quantity unitPrice currencyCode notes createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ createTransaction: Transaction }>(
    mutation,
    {
      input: {
        accountId,
        assetId,
        transactionType,
        tradeDate,
        quantity,
        unitPrice,
        currencyCode,
        notes,
      },
    },
  );
  return data.createTransaction;
}

export async function updateTransaction(
  id: number,
  accountId: number,
  assetId: number,
  transactionType: TransactionType,
  tradeDate: string,
  quantity: string,
  unitPrice: string,
  currencyCode: string,
  notes: string | null,
): Promise<Transaction> {
  const mutation = gql`
    mutation UpdateTransaction($id: Int!, $input: UpdateTransactionInput!) {
      updateTransaction(id: $id, input: $input) {
        id accountId assetId transactionType tradeDate
        quantity unitPrice currencyCode notes createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ updateTransaction: Transaction }>(
    mutation,
    {
      id,
      input: {
        accountId,
        assetId,
        transactionType,
        tradeDate,
        quantity,
        unitPrice,
        currencyCode,
        notes,
      },
    },
  );
  return data.updateTransaction;
}

export async function deleteTransaction(id: number): Promise<void> {
  const mutation = gql`
    mutation DeleteTransaction($id: Int!) {
      deleteTransaction(id: $id)
    }
  `;
  await client().request(mutation, { id });
}
