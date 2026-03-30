export type Maybe<T> = T | null;
export type InputMaybe<T> = T | null;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string; }
  String: { input: string; output: string; }
  Boolean: { input: boolean; output: boolean; }
  Int: { input: number; output: number; }
  Float: { input: number; output: number; }
};

export type AccountDetail = {
  __typename?: 'AccountDetail';
  accountType: AccountType;
  assetTotalAmount: Maybe<Scalars['String']['output']>;
  balances: Array<Balance>;
  baseCurrency: Scalars['String']['output'];
  cashTotalAmount: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  id: Scalars['Int']['output'];
  name: Scalars['String']['output'];
  summaryStatus: SummaryStatus;
  totalAmount: Maybe<Scalars['String']['output']>;
  totalCurrency: Maybe<Scalars['String']['output']>;
};

export type AccountInput = {
  accountType: AccountType;
  baseCurrency: Scalars['String']['input'];
  name: Scalars['String']['input'];
};

export type AccountSummary = {
  __typename?: 'AccountSummary';
  accountType: AccountType;
  assetTotalAmount: Maybe<Scalars['String']['output']>;
  baseCurrency: Scalars['String']['output'];
  cashTotalAmount: Maybe<Scalars['String']['output']>;
  id: Scalars['Int']['output'];
  name: Scalars['String']['output'];
  summaryStatus: SummaryStatus;
  totalAmount: Maybe<Scalars['String']['output']>;
  totalCurrency: Maybe<Scalars['String']['output']>;
};

export type AccountType =
  | 'BANK'
  | 'BROKER'
  | 'CRYPTO';

export type Asset = {
  __typename?: 'Asset';
  assetType: AssetType;
  avgCostBasis: Maybe<Scalars['String']['output']>;
  avgCostBasisCurrency: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  currentPrice: Maybe<Scalars['String']['output']>;
  currentPriceAsOf: Maybe<Scalars['String']['output']>;
  currentPriceCurrency: Maybe<Scalars['String']['output']>;
  id: Scalars['Int']['output'];
  isin: Maybe<Scalars['String']['output']>;
  name: Scalars['String']['output'];
  previousClose: Maybe<Scalars['String']['output']>;
  previousCloseCurrency: Maybe<Scalars['String']['output']>;
  quoteSymbol: Maybe<Scalars['String']['output']>;
  symbol: Scalars['String']['output'];
  totalQuantity: Maybe<Scalars['String']['output']>;
  updatedAt: Scalars['String']['output'];
};

export type AssetInput = {
  assetType: AssetType;
  isin?: InputMaybe<Scalars['String']['input']>;
  name: Scalars['String']['input'];
  quoteSymbol?: InputMaybe<Scalars['String']['input']>;
  symbol: Scalars['String']['input'];
};

export type AssetPosition = {
  __typename?: 'AssetPosition';
  accountId: Scalars['Int']['output'];
  assetId: Scalars['Int']['output'];
  quantity: Scalars['String']['output'];
};

export type AssetType =
  | 'BOND'
  | 'CASH_EQUIVALENT'
  | 'CRYPTO'
  | 'ETF'
  | 'OTHER'
  | 'STOCK';

export type Balance = {
  __typename?: 'Balance';
  amount: Scalars['String']['output'];
  currency: Scalars['String']['output'];
  updatedAt: Scalars['String']['output'];
};

export type FxRateSummary = {
  __typename?: 'FxRateSummary';
  lastUpdated: Maybe<Scalars['String']['output']>;
  rates: Array<FxRateSummaryItem>;
  refreshError: Maybe<Scalars['String']['output']>;
  refreshStatus: RefreshAvailability;
  targetCurrency: Scalars['String']['output'];
};

export type FxRateSummaryItem = {
  __typename?: 'FxRateSummaryItem';
  currency: Scalars['String']['output'];
  rate: Scalars['String']['output'];
};

export type MutationRoot = {
  __typename?: 'MutationRoot';
  createAccount: AccountDetail;
  createAsset: Asset;
  createTransaction: Transaction;
  createTransfer: Transfer;
  deleteAccount: Scalars['Int']['output'];
  deleteAsset: Scalars['Int']['output'];
  deleteBalance: Scalars['Boolean']['output'];
  deleteTransaction: Scalars['Int']['output'];
  deleteTransfer: Scalars['Int']['output'];
  updateAccount: AccountDetail;
  updateAsset: Asset;
  updateTransaction: Transaction;
  upsertBalance: Balance;
};


export type MutationRootCreateAccountArgs = {
  input: AccountInput;
};


export type MutationRootCreateAssetArgs = {
  input: AssetInput;
};


export type MutationRootCreateTransactionArgs = {
  input: TransactionInput;
};


export type MutationRootCreateTransferArgs = {
  input: TransferInput;
};


export type MutationRootDeleteAccountArgs = {
  id: Scalars['Int']['input'];
};


export type MutationRootDeleteAssetArgs = {
  id: Scalars['Int']['input'];
};


export type MutationRootDeleteBalanceArgs = {
  accountId: Scalars['Int']['input'];
  currency: Scalars['String']['input'];
};


export type MutationRootDeleteTransactionArgs = {
  id: Scalars['Int']['input'];
};


export type MutationRootDeleteTransferArgs = {
  id: Scalars['Int']['input'];
};


export type MutationRootUpdateAccountArgs = {
  id: Scalars['Int']['input'];
  input: AccountInput;
};


export type MutationRootUpdateAssetArgs = {
  id: Scalars['Int']['input'];
  input: AssetInput;
};


export type MutationRootUpdateTransactionArgs = {
  id: Scalars['Int']['input'];
  input: TransactionInput;
};


export type MutationRootUpsertBalanceArgs = {
  accountId: Scalars['Int']['input'];
  input: UpsertBalanceInput;
};

export type PortfolioAccountTotal = {
  __typename?: 'PortfolioAccountTotal';
  accountType: AccountType;
  assetTotalAmount: Maybe<Scalars['String']['output']>;
  cashTotalAmount: Maybe<Scalars['String']['output']>;
  id: Scalars['Int']['output'];
  name: Scalars['String']['output'];
  summaryStatus: SummaryStatus;
  totalAmount: Maybe<Scalars['String']['output']>;
  totalCurrency: Scalars['String']['output'];
};

export type PortfolioAllocationSlice = {
  __typename?: 'PortfolioAllocationSlice';
  amount: Scalars['String']['output'];
  label: Scalars['String']['output'];
};

export type PortfolioCashByCurrency = {
  __typename?: 'PortfolioCashByCurrency';
  amount: Scalars['String']['output'];
  convertedAmount: Maybe<Scalars['String']['output']>;
  currency: Scalars['String']['output'];
};

export type PortfolioHolding = {
  __typename?: 'PortfolioHolding';
  assetId: Scalars['Int']['output'];
  name: Scalars['String']['output'];
  symbol: Scalars['String']['output'];
  value: Scalars['String']['output'];
};

export type PortfolioSnapshot = {
  __typename?: 'PortfolioSnapshot';
  currency: Scalars['String']['output'];
  recordedAt: Scalars['String']['output'];
  totalValue: Scalars['String']['output'];
};

export type PortfolioSummary = {
  __typename?: 'PortfolioSummary';
  accountTotals: Array<PortfolioAccountTotal>;
  allocationIsPartial: Scalars['Boolean']['output'];
  allocationTotals: Array<PortfolioAllocationSlice>;
  cashByCurrency: Array<PortfolioCashByCurrency>;
  displayCurrency: Scalars['String']['output'];
  fxLastUpdated: Maybe<Scalars['String']['output']>;
  fxRefreshError: Maybe<Scalars['String']['output']>;
  fxRefreshStatus: RefreshAvailability;
  holdings: Array<PortfolioHolding>;
  holdingsIsPartial: Scalars['Boolean']['output'];
  totalValueAmount: Maybe<Scalars['String']['output']>;
  totalValueStatus: SummaryStatus;
};

export type QueryRoot = {
  __typename?: 'QueryRoot';
  account: AccountDetail;
  accountPositions: Array<AssetPosition>;
  accounts: Array<AccountSummary>;
  asset: Asset;
  assets: Array<Asset>;
  currencies: Array<Scalars['String']['output']>;
  fxRates: FxRateSummary;
  portfolio: PortfolioSummary;
  portfolioHistory: Array<PortfolioSnapshot>;
  transaction: Transaction;
  transactions: Array<Transaction>;
  transfers: Array<Transfer>;
};


export type QueryRootAccountArgs = {
  id: Scalars['Int']['input'];
};


export type QueryRootAccountPositionsArgs = {
  accountId: Scalars['Int']['input'];
};


export type QueryRootAssetArgs = {
  id: Scalars['Int']['input'];
};


export type QueryRootTransactionArgs = {
  id: Scalars['Int']['input'];
};


export type QueryRootTransactionsArgs = {
  accountId?: InputMaybe<Scalars['Int']['input']>;
};

export type RefreshAvailability =
  | 'AVAILABLE'
  | 'UNAVAILABLE';

export type SummaryStatus =
  | 'CONVERSION_UNAVAILABLE'
  | 'OK';

export type Transaction = {
  __typename?: 'Transaction';
  accountId: Scalars['Int']['output'];
  assetId: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  currencyCode: Scalars['String']['output'];
  id: Scalars['Int']['output'];
  notes: Maybe<Scalars['String']['output']>;
  quantity: Scalars['String']['output'];
  tradeDate: Scalars['String']['output'];
  transactionType: TransactionType;
  unitPrice: Scalars['String']['output'];
  updatedAt: Scalars['String']['output'];
};

export type TransactionInput = {
  accountId: Scalars['Int']['input'];
  assetId: Scalars['Int']['input'];
  currencyCode: Scalars['String']['input'];
  notes?: InputMaybe<Scalars['String']['input']>;
  quantity: Scalars['String']['input'];
  tradeDate: Scalars['String']['input'];
  transactionType: TransactionType;
  unitPrice: Scalars['String']['input'];
};

export type TransactionType =
  | 'BUY'
  | 'SELL';

export type Transfer = {
  __typename?: 'Transfer';
  createdAt: Scalars['String']['output'];
  fromAccountId: Scalars['Int']['output'];
  fromAmount: Scalars['String']['output'];
  fromCurrency: Scalars['String']['output'];
  id: Scalars['Int']['output'];
  notes: Maybe<Scalars['String']['output']>;
  toAccountId: Scalars['Int']['output'];
  toAmount: Scalars['String']['output'];
  toCurrency: Scalars['String']['output'];
  transferDate: Scalars['String']['output'];
};

export type TransferInput = {
  fromAccountId: Scalars['Int']['input'];
  fromAmount: Scalars['String']['input'];
  fromCurrency: Scalars['String']['input'];
  notes?: InputMaybe<Scalars['String']['input']>;
  toAccountId: Scalars['Int']['input'];
  toAmount: Scalars['String']['input'];
  toCurrency: Scalars['String']['input'];
  transferDate: Scalars['String']['input'];
};

export type UpsertBalanceInput = {
  amount: Scalars['String']['input'];
  currency: Scalars['String']['input'];
};

export type AccountPositionsQueryVariables = Exact<{
  accountId: Scalars['Int']['input'];
}>;


export type AccountPositionsQuery = { __typename?: 'QueryRoot', accountPositions: Array<{ __typename?: 'AssetPosition', accountId: number, assetId: number, quantity: string }> };

export type AccountAssetsQueryVariables = Exact<{ [key: string]: never; }>;


export type AccountAssetsQuery = { __typename?: 'QueryRoot', assets: Array<{ __typename?: 'Asset', id: number, symbol: string, name: string, assetType: AssetType, currentPrice: string | null, currentPriceCurrency: string | null }> };

export type AccountFxRatesQueryVariables = Exact<{ [key: string]: never; }>;


export type AccountFxRatesQuery = { __typename?: 'QueryRoot', fxRates: { __typename?: 'FxRateSummary', targetCurrency: string, rates: Array<{ __typename?: 'FxRateSummaryItem', currency: string, rate: string }> } };

export type AccountBalancesQueryVariables = Exact<{
  id: Scalars['Int']['input'];
}>;


export type AccountBalancesQuery = { __typename?: 'QueryRoot', account: { __typename?: 'AccountDetail', id: number, baseCurrency: string, balances: Array<{ __typename?: 'Balance', currency: string, amount: string, updatedAt: string }> } };

export type BalanceCurrenciesQueryVariables = Exact<{ [key: string]: never; }>;


export type BalanceCurrenciesQuery = { __typename?: 'QueryRoot', currencies: Array<string> };

export type UpsertBalanceMutationVariables = Exact<{
  accountId: Scalars['Int']['input'];
  input: UpsertBalanceInput;
}>;


export type UpsertBalanceMutation = { __typename?: 'MutationRoot', upsertBalance: { __typename?: 'Balance', currency: string, amount: string, updatedAt: string } };

export type DeleteBalanceMutationVariables = Exact<{
  accountId: Scalars['Int']['input'];
  currency: Scalars['String']['input'];
}>;


export type DeleteBalanceMutation = { __typename?: 'MutationRoot', deleteBalance: boolean };

export type AccountQueryVariables = Exact<{
  id: Scalars['Int']['input'];
}>;


export type AccountQuery = { __typename?: 'QueryRoot', account: { __typename?: 'AccountDetail', id: number, name: string, accountType: AccountType, baseCurrency: string, summaryStatus: SummaryStatus, createdAt: string, cashTotalAmount: string | null, assetTotalAmount: string | null, totalAmount: string | null, totalCurrency: string | null, balances: Array<{ __typename?: 'Balance', currency: string, amount: string, updatedAt: string }> } };

export type DeleteAccountMutationVariables = Exact<{
  id: Scalars['Int']['input'];
}>;


export type DeleteAccountMutation = { __typename?: 'MutationRoot', deleteAccount: number };

export type NewAccountCurrenciesQueryVariables = Exact<{ [key: string]: never; }>;


export type NewAccountCurrenciesQuery = { __typename?: 'QueryRoot', currencies: Array<string> };

export type CreateAccountMutationVariables = Exact<{
  input: AccountInput;
}>;


export type CreateAccountMutation = { __typename?: 'MutationRoot', createAccount: { __typename?: 'AccountDetail', id: number, name: string, accountType: AccountType, baseCurrency: string } };

export type AccountsListQueryVariables = Exact<{ [key: string]: never; }>;


export type AccountsListQuery = { __typename?: 'QueryRoot', accounts: Array<{ __typename?: 'AccountSummary', id: number, name: string, accountType: AccountType, baseCurrency: string, summaryStatus: SummaryStatus, cashTotalAmount: string | null, assetTotalAmount: string | null, totalAmount: string | null, totalCurrency: string | null }> };

export type AccountsListPortfolioQueryVariables = Exact<{ [key: string]: never; }>;


export type AccountsListPortfolioQuery = { __typename?: 'QueryRoot', portfolio: { __typename?: 'PortfolioSummary', displayCurrency: string, totalValueAmount: string | null, accountTotals: Array<{ __typename?: 'PortfolioAccountTotal', id: number, summaryStatus: SummaryStatus, cashTotalAmount: string | null, assetTotalAmount: string | null, totalAmount: string | null }> } };

export type CreateAssetMutationVariables = Exact<{
  input: AssetInput;
}>;


export type CreateAssetMutation = { __typename?: 'MutationRoot', createAsset: { __typename?: 'Asset', id: number, symbol: string, name: string, assetType: AssetType, quoteSymbol: string | null, isin: string | null, currentPrice: string | null, currentPriceCurrency: string | null, currentPriceAsOf: string | null, totalQuantity: string | null } };

export type UpdateAssetMutationVariables = Exact<{
  id: Scalars['Int']['input'];
  input: AssetInput;
}>;


export type UpdateAssetMutation = { __typename?: 'MutationRoot', updateAsset: { __typename?: 'Asset', id: number, symbol: string, name: string, assetType: AssetType, quoteSymbol: string | null, isin: string | null, currentPrice: string | null, currentPriceCurrency: string | null, currentPriceAsOf: string | null, totalQuantity: string | null } };

export type DeleteAssetMutationVariables = Exact<{
  id: Scalars['Int']['input'];
}>;


export type DeleteAssetMutation = { __typename?: 'MutationRoot', deleteAsset: number };

export type AssetsQueryVariables = Exact<{ [key: string]: never; }>;


export type AssetsQuery = { __typename?: 'QueryRoot', assets: Array<{ __typename?: 'Asset', id: number, symbol: string, name: string, assetType: AssetType, quoteSymbol: string | null, isin: string | null, currentPrice: string | null, currentPriceCurrency: string | null, currentPriceAsOf: string | null, totalQuantity: string | null, avgCostBasis: string | null, avgCostBasisCurrency: string | null, previousClose: string | null, previousCloseCurrency: string | null }> };

export type FxRatesQueryVariables = Exact<{ [key: string]: never; }>;


export type FxRatesQuery = { __typename?: 'QueryRoot', fxRates: { __typename?: 'FxRateSummary', targetCurrency: string, lastUpdated: string | null, refreshStatus: RefreshAvailability, refreshError: string | null, rates: Array<{ __typename?: 'FxRateSummaryItem', currency: string, rate: string }> } };

export type PortfolioHistoryQueryVariables = Exact<{ [key: string]: never; }>;


export type PortfolioHistoryQuery = { __typename?: 'QueryRoot', portfolioHistory: Array<{ __typename?: 'PortfolioSnapshot', totalValue: string, currency: string, recordedAt: string }> };

export type PortfolioQueryVariables = Exact<{ [key: string]: never; }>;


export type PortfolioQuery = { __typename?: 'QueryRoot', portfolio: { __typename?: 'PortfolioSummary', displayCurrency: string, totalValueStatus: SummaryStatus, totalValueAmount: string | null, fxLastUpdated: string | null, fxRefreshStatus: RefreshAvailability, fxRefreshError: string | null, allocationIsPartial: boolean, holdingsIsPartial: boolean, accountTotals: Array<{ __typename?: 'PortfolioAccountTotal', id: number, name: string, accountType: AccountType, summaryStatus: SummaryStatus, cashTotalAmount: string | null, assetTotalAmount: string | null, totalAmount: string | null, totalCurrency: string }>, cashByCurrency: Array<{ __typename?: 'PortfolioCashByCurrency', currency: string, amount: string, convertedAmount: string | null }>, allocationTotals: Array<{ __typename?: 'PortfolioAllocationSlice', label: string, amount: string }>, holdings: Array<{ __typename?: 'PortfolioHolding', assetId: number, symbol: string, name: string, value: string }> } };

export type CreateTransactionMutationVariables = Exact<{
  input: TransactionInput;
}>;


export type CreateTransactionMutation = { __typename?: 'MutationRoot', createTransaction: { __typename?: 'Transaction', id: number, accountId: number, assetId: number, transactionType: TransactionType, tradeDate: string, quantity: string, unitPrice: string, currencyCode: string, notes: string | null } };

export type UpdateTransactionMutationVariables = Exact<{
  id: Scalars['Int']['input'];
  input: TransactionInput;
}>;


export type UpdateTransactionMutation = { __typename?: 'MutationRoot', updateTransaction: { __typename?: 'Transaction', id: number, accountId: number, assetId: number, transactionType: TransactionType, tradeDate: string, quantity: string, unitPrice: string, currencyCode: string, notes: string | null } };

export type TransactionAccountsQueryVariables = Exact<{ [key: string]: never; }>;


export type TransactionAccountsQuery = { __typename?: 'QueryRoot', accounts: Array<{ __typename?: 'AccountSummary', id: number, name: string, accountType: AccountType, baseCurrency: string }> };

export type TransactionAssetsQueryVariables = Exact<{ [key: string]: never; }>;


export type TransactionAssetsQuery = { __typename?: 'QueryRoot', assets: Array<{ __typename?: 'Asset', id: number, symbol: string, name: string, assetType: AssetType }> };

export type TransactionsQueryVariables = Exact<{
  accountId?: InputMaybe<Scalars['Int']['input']>;
}>;


export type TransactionsQuery = { __typename?: 'QueryRoot', transactions: Array<{ __typename?: 'Transaction', id: number, accountId: number, assetId: number, transactionType: TransactionType, tradeDate: string, quantity: string, unitPrice: string, currencyCode: string, notes: string | null }> };

export type DeleteTransactionMutationVariables = Exact<{
  id: Scalars['Int']['input'];
}>;


export type DeleteTransactionMutation = { __typename?: 'MutationRoot', deleteTransaction: number };

export type CreateTransferMutationVariables = Exact<{
  input: TransferInput;
}>;


export type CreateTransferMutation = { __typename?: 'MutationRoot', createTransfer: { __typename?: 'Transfer', id: number, fromAccountId: number, toAccountId: number, fromCurrency: string, fromAmount: string, toCurrency: string, toAmount: string, transferDate: string, notes: string | null } };

export type TransferAccountsQueryVariables = Exact<{ [key: string]: never; }>;


export type TransferAccountsQuery = { __typename?: 'QueryRoot', accounts: Array<{ __typename?: 'AccountSummary', id: number, name: string, baseCurrency: string }> };

export type TransfersQueryVariables = Exact<{ [key: string]: never; }>;


export type TransfersQuery = { __typename?: 'QueryRoot', transfers: Array<{ __typename?: 'Transfer', id: number, fromAccountId: number, toAccountId: number, fromCurrency: string, fromAmount: string, toCurrency: string, toAmount: string, transferDate: string, notes: string | null }> };

export type DeleteTransferMutationVariables = Exact<{
  id: Scalars['Int']['input'];
}>;


export type DeleteTransferMutation = { __typename?: 'MutationRoot', deleteTransfer: number };
