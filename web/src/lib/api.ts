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

export type FxRateSummaryItem = {
  currency: string;
  rate: string;
};

export type FxRateSummary = {
  targetCurrency: string;
  rates: FxRateSummaryItem[];
  lastUpdated: string | null;
  refreshStatus: string;
  refreshError: string | null;
};

export type PortfolioAccountTotal = {
  id: number;
  name: string;
  accountType: string;
  summaryStatus: string;
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
  totalValueStatus: string;
  totalValueAmount: string | null;
  accountTotals: PortfolioAccountTotal[];
  cashByCurrency: PortfolioCashByCurrency[];
  fxLastUpdated: string | null;
  fxRefreshStatus: string;
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
  accountType: string;
  baseCurrency: string;
  summaryStatus: string;
  cashTotalAmount: string | null;
  assetTotalAmount: string | null;
  totalAmount: string | null;
  totalCurrency: string | null;
};

export type AccountDetail = {
  id: number;
  name: string;
  accountType: string;
  baseCurrency: string;
  summaryStatus: string;
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
  assetType: string;
  quoteSymbol: string | null;
  isin: string | null;
  currentPrice: string | null;
  currentPriceCurrency: string | null;
  currentPriceAsOf: string | null;
  totalQuantity: string | null;
  createdAt?: string | null;
  updatedAt?: string | null;
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
  transactionType: string;
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
  accountType: string,
  baseCurrency: string,
): Promise<AccountDetail> {
  const mutation = gql`
    mutation CreateAccount(
      $name: String!
      $accountType: String!
      $baseCurrency: String!
    ) {
      createAccount(
        name: $name
        accountType: $accountType
        baseCurrency: $baseCurrency
      ) {
        id name accountType baseCurrency summaryStatus createdAt
        cashTotalAmount assetTotalAmount totalAmount totalCurrency
        balances { currency amount updatedAt }
      }
    }
  `;
  const data = await client().request<{ createAccount: AccountDetail }>(
    mutation,
    { name, accountType, baseCurrency },
  );
  return data.createAccount;
}

export async function updateAccount(
  id: number,
  name: string,
  accountType: string,
  baseCurrency: string,
): Promise<AccountDetail> {
  const mutation = gql`
    mutation UpdateAccount(
      $id: Int!
      $name: String!
      $accountType: String!
      $baseCurrency: String!
    ) {
      updateAccount(
        id: $id
        name: $name
        accountType: $accountType
        baseCurrency: $baseCurrency
      ) {
        id name accountType baseCurrency summaryStatus createdAt
        cashTotalAmount assetTotalAmount totalAmount totalCurrency
        balances { currency amount updatedAt }
      }
    }
  `;
  const data = await client().request<{ updateAccount: AccountDetail }>(
    mutation,
    { id, name, accountType, baseCurrency },
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
    mutation UpsertBalance(
      $accountId: Int!
      $currency: String!
      $amount: String!
    ) {
      upsertBalance(accountId: $accountId, currency: $currency, amount: $amount) {
        currency amount updatedAt
      }
    }
  `;
  const data = await client().request<{ upsertBalance: Balance }>(mutation, {
    accountId,
    currency,
    amount,
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
  assetType: string,
  quoteSymbol?: string | null,
  isin?: string | null,
): Promise<Asset> {
  const mutation = gql`
    mutation CreateAsset(
      $symbol: String!
      $name: String!
      $assetType: String!
      $quoteSymbol: String
      $isin: String
    ) {
      createAsset(
        symbol: $symbol
        name: $name
        assetType: $assetType
        quoteSymbol: $quoteSymbol
        isin: $isin
      ) {
        id symbol name assetType quoteSymbol isin
        currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
        createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ createAsset: Asset }>(mutation, {
    symbol,
    name,
    assetType,
    quoteSymbol: quoteSymbol ?? null,
    isin: isin ?? null,
  });
  return data.createAsset;
}

export async function updateAsset(
  id: number,
  symbol: string,
  name: string,
  assetType: string,
  quoteSymbol?: string | null,
  isin?: string | null,
): Promise<Asset> {
  const mutation = gql`
    mutation UpdateAsset(
      $id: Int!
      $symbol: String!
      $name: String!
      $assetType: String!
      $quoteSymbol: String
      $isin: String
    ) {
      updateAsset(
        id: $id
        symbol: $symbol
        name: $name
        assetType: $assetType
        quoteSymbol: $quoteSymbol
        isin: $isin
      ) {
        id symbol name assetType quoteSymbol isin
        currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
        createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ updateAsset: Asset }>(mutation, {
    id,
    symbol,
    name,
    assetType,
    quoteSymbol: quoteSymbol ?? null,
    isin: isin ?? null,
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
  transactionType: string,
  tradeDate: string,
  quantity: string,
  unitPrice: string,
  currencyCode: string,
  notes: string | null,
): Promise<Transaction> {
  const mutation = gql`
    mutation CreateTransaction(
      $accountId: Int!
      $assetId: Int!
      $transactionType: String!
      $tradeDate: String!
      $quantity: String!
      $unitPrice: String!
      $currencyCode: String!
      $notes: String
    ) {
      createTransaction(
        accountId: $accountId
        assetId: $assetId
        transactionType: $transactionType
        tradeDate: $tradeDate
        quantity: $quantity
        unitPrice: $unitPrice
        currencyCode: $currencyCode
        notes: $notes
      ) {
        id accountId assetId transactionType tradeDate
        quantity unitPrice currencyCode notes createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ createTransaction: Transaction }>(
    mutation,
    {
      accountId,
      assetId,
      transactionType,
      tradeDate,
      quantity,
      unitPrice,
      currencyCode,
      notes,
    },
  );
  return data.createTransaction;
}

export async function updateTransaction(
  id: number,
  accountId: number,
  assetId: number,
  transactionType: string,
  tradeDate: string,
  quantity: string,
  unitPrice: string,
  currencyCode: string,
  notes: string | null,
): Promise<Transaction> {
  const mutation = gql`
    mutation UpdateTransaction(
      $id: Int!
      $accountId: Int!
      $assetId: Int!
      $transactionType: String!
      $tradeDate: String!
      $quantity: String!
      $unitPrice: String!
      $currencyCode: String!
      $notes: String
    ) {
      updateTransaction(
        id: $id
        accountId: $accountId
        assetId: $assetId
        transactionType: $transactionType
        tradeDate: $tradeDate
        quantity: $quantity
        unitPrice: $unitPrice
        currencyCode: $currencyCode
        notes: $notes
      ) {
        id accountId assetId transactionType tradeDate
        quantity unitPrice currencyCode notes createdAt updatedAt
      }
    }
  `;
  const data = await client().request<{ updateTransaction: Transaction }>(
    mutation,
    {
      id,
      accountId,
      assetId,
      transactionType,
      tradeDate,
      quantity,
      unitPrice,
      currencyCode,
      notes,
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
