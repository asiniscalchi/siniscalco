/** Internal type. DO NOT USE DIRECTLY. */
type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
/** Internal type. DO NOT USE DIRECTLY. */
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
export type AccountInput = {
  accountType: AccountType;
  baseCurrency: string;
  name: string;
};

export type AccountType =
  | 'BANK'
  | 'BROKER'
  | 'CRYPTO';

export type AssetInput = {
  assetType: AssetType;
  isin?: string | null | undefined;
  name: string;
  quoteSymbol?: string | null | undefined;
  symbol: string;
};

export type AssetType =
  | 'BOND'
  | 'CASH_EQUIVALENT'
  | 'CRYPTO'
  | 'ETF'
  | 'OTHER'
  | 'STOCK';

export type CashMovementInput = {
  amount: string;
  currency: string;
  date: string;
  notes?: string | null | undefined;
};

export type RefreshAvailability =
  | 'AVAILABLE'
  | 'UNAVAILABLE';

export type SummaryStatus =
  | 'CONVERSION_UNAVAILABLE'
  | 'OK';

export type TodoInput = {
  title: string;
};

export type TransactionInput = {
  accountId: number;
  assetId: number;
  currencyCode: string;
  notes?: string | null | undefined;
  quantity: string;
  tradeDate: string;
  transactionType: TransactionType;
  unitPrice: string;
};

export type TransactionType =
  | 'BUY'
  | 'OPENING'
  | 'SELL';

export type TransferInput = {
  fromAccountId: number;
  fromAmount: string;
  fromCurrency: string;
  notes?: string | null | undefined;
  toAccountId: number;
  toAmount: string;
  toCurrency: string;
  transferDate: string;
};

export type TodosNavQueryVariables = Exact<{ [key: string]: never; }>;


export type TodosNavQuery = { todos: Array<{ id: number, completed: boolean }> };

export type AccountPositionsQueryVariables = Exact<{
  accountId: number;
}>;


export type AccountPositionsQuery = { accountPositions: Array<{ accountId: number, assetId: number, quantity: string }> };

export type AccountAssetsQueryVariables = Exact<{ [key: string]: never; }>;


export type AccountAssetsQuery = { assets: Array<{ id: number, symbol: string, name: string, assetType: AssetType, currentPrice: string | null, currentPriceCurrency: string | null }> };

export type AccountFxRatesQueryVariables = Exact<{ [key: string]: never; }>;


export type AccountFxRatesQuery = { fxRates: { targetCurrency: string, rates: Array<{ currency: string, rate: string }> } };

export type AccountBalancesQueryVariables = Exact<{
  id: number;
}>;


export type AccountBalancesQuery = { account: { id: number, balances: Array<{ currency: string, amount: string, updatedAt: string }> } };

export type AccountQueryVariables = Exact<{
  id: number;
}>;


export type AccountQuery = { account: { id: number, name: string, accountType: AccountType, baseCurrency: string, summaryStatus: SummaryStatus, createdAt: string, cashTotalAmount: string | null, assetTotalAmount: string | null, totalAmount: string | null, totalCurrency: string | null, balances: Array<{ currency: string, amount: string, updatedAt: string }> } };

export type DeleteAccountMutationVariables = Exact<{
  id: number;
}>;


export type DeleteAccountMutation = { deleteAccount: number };

export type NewAccountCurrenciesQueryVariables = Exact<{ [key: string]: never; }>;


export type NewAccountCurrenciesQuery = { currencies: Array<string> };

export type CreateAccountMutationVariables = Exact<{
  input: AccountInput;
}>;


export type CreateAccountMutation = { createAccount: { id: number, name: string, accountType: AccountType, baseCurrency: string } };

export type AccountsListQueryVariables = Exact<{ [key: string]: never; }>;


export type AccountsListQuery = { accounts: Array<{ id: number, name: string, accountType: AccountType, baseCurrency: string, summaryStatus: SummaryStatus, cashTotalAmount: string | null, assetTotalAmount: string | null, totalAmount: string | null, totalCurrency: string | null }> };

export type AccountsListPortfolioQueryVariables = Exact<{ [key: string]: never; }>;


export type AccountsListPortfolioQuery = { portfolio: { displayCurrency: string, totalValueAmount: string | null, accountTotals: Array<{ id: number, summaryStatus: SummaryStatus, cashTotalAmount: string | null, assetTotalAmount: string | null, totalAmount: string | null }> } };

export type TransactionAccountsQueryVariables = Exact<{ [key: string]: never; }>;


export type TransactionAccountsQuery = { accounts: Array<{ id: number, name: string, accountType: AccountType, baseCurrency: string }> };

export type TransactionAssetsQueryVariables = Exact<{ [key: string]: never; }>;


export type TransactionAssetsQuery = { assets: Array<{ id: number, symbol: string, name: string, assetType: AssetType }> };

export type TransactionsQueryVariables = Exact<{
  accountId?: number | null | undefined;
}>;


export type TransactionsQuery = { transactions: Array<{ id: number, accountId: number, assetId: number, transactionType: TransactionType, tradeDate: string, quantity: string, unitPrice: string, currencyCode: string, notes: string | null }> };

export type ActivityCashMovementsQueryVariables = Exact<{
  accountId?: number | null | undefined;
}>;


export type ActivityCashMovementsQuery = { cashMovements: Array<{ id: number, accountId: number, currency: string, amount: string, date: string, notes: string | null }> };

export type ActivityTransfersQueryVariables = Exact<{
  accountId?: number | null | undefined;
}>;


export type ActivityTransfersQuery = { transfers: Array<{ id: number, fromAccountId: number, toAccountId: number, fromCurrency: string, fromAmount: string, toCurrency: string, toAmount: string, transferDate: string, notes: string | null }> };

export type DeleteTransactionMutationVariables = Exact<{
  id: number;
}>;


export type DeleteTransactionMutation = { deleteTransaction: number };

export type BalanceCurrenciesQueryVariables = Exact<{ [key: string]: never; }>;


export type BalanceCurrenciesQuery = { currencies: Array<string> };

export type CreateCashMovementMutationVariables = Exact<{
  accountId: number;
  input: CashMovementInput;
}>;


export type CreateCashMovementMutation = { createCashMovement: { currency: string, amount: string, date: string } };

export type CreateTransactionMutationVariables = Exact<{
  input: TransactionInput;
}>;


export type CreateTransactionMutation = { createTransaction: { id: number, accountId: number, assetId: number, transactionType: TransactionType, tradeDate: string, quantity: string, unitPrice: string, currencyCode: string, notes: string | null } };

export type UpdateTransactionMutationVariables = Exact<{
  id: number;
  input: TransactionInput;
}>;


export type UpdateTransactionMutation = { updateTransaction: { id: number, accountId: number, assetId: number, transactionType: TransactionType, tradeDate: string, quantity: string, unitPrice: string, currencyCode: string, notes: string | null } };

export type CreateAssetMutationVariables = Exact<{
  input: AssetInput;
}>;


export type CreateAssetMutation = { createAsset: { id: number, symbol: string, name: string, assetType: AssetType, quoteSymbol: string | null, isin: string | null, currentPrice: string | null, currentPriceCurrency: string | null, currentPriceAsOf: string | null, totalQuantity: string | null } };

export type UpdateAssetMutationVariables = Exact<{
  id: number;
  input: AssetInput;
}>;


export type UpdateAssetMutation = { updateAsset: { id: number, symbol: string, name: string, assetType: AssetType, quoteSymbol: string | null, isin: string | null, currentPrice: string | null, currentPriceCurrency: string | null, currentPriceAsOf: string | null, totalQuantity: string | null } };

export type DeleteAssetMutationVariables = Exact<{
  id: number;
}>;


export type DeleteAssetMutation = { deleteAsset: number };

export type AssetsQueryVariables = Exact<{ [key: string]: never; }>;


export type AssetsQuery = { assets: Array<{ id: number, symbol: string, name: string, assetType: AssetType, quoteSymbol: string | null, isin: string | null, quoteSourceSymbol: string | null, quoteSourceProvider: string | null, quoteSourceLastSuccessAt: string | null, currentPrice: string | null, currentPriceCurrency: string | null, currentPriceAsOf: string | null, totalQuantity: string | null, avgCostBasis: string | null, avgCostBasisCurrency: string | null, previousClose: string | null, previousCloseCurrency: string | null, convertedTotalValue: string | null, convertedTotalValueCurrency: string | null, convertedTotalCostBasis: string | null, convertedTotalCostBasisCurrency: string | null }> };

export type FxRatesQueryVariables = Exact<{ [key: string]: never; }>;


export type FxRatesQuery = { fxRates: { targetCurrency: string, lastUpdated: string | null, refreshStatus: RefreshAvailability, refreshError: string | null, rates: Array<{ currency: string, rate: string }> } };

export type PortfolioHistoryQueryVariables = Exact<{ [key: string]: never; }>;


export type PortfolioHistoryQuery = { portfolioHistory: Array<{ totalValue: string, currency: string, recordedAt: string }> };

export type PortfolioQueryVariables = Exact<{ [key: string]: never; }>;


export type PortfolioQuery = { portfolio: { displayCurrency: string, totalValueStatus: SummaryStatus, totalValueAmount: string | null, gain24hAmount: string | null, totalGainAmount: string | null, fxLastUpdated: string | null, fxRefreshStatus: RefreshAvailability, fxRefreshError: string | null, allocationIsPartial: boolean, holdingsIsPartial: boolean, accountTotals: Array<{ id: number, name: string, accountType: AccountType, summaryStatus: SummaryStatus, cashTotalAmount: string | null, assetTotalAmount: string | null, totalAmount: string | null, totalCurrency: string }>, cashByCurrency: Array<{ currency: string, amount: string, convertedAmount: string | null }>, allocationTotals: Array<{ label: string, amount: string }>, holdings: Array<{ assetId: number | null, symbol: string, name: string, value: string, gain24hAmount: string | null }> } };

export type TodosQueryVariables = Exact<{ [key: string]: never; }>;


export type TodosQuery = { todos: Array<{ id: number, title: string, completed: boolean, createdAt: string, updatedAt: string }> };

export type CreateTodoMutationVariables = Exact<{
  input: TodoInput;
}>;


export type CreateTodoMutation = { createTodo: { id: number, title: string, completed: boolean, createdAt: string, updatedAt: string } };

export type UpdateTodoCompletedMutationVariables = Exact<{
  id: number;
  completed: boolean;
}>;


export type UpdateTodoCompletedMutation = { updateTodoCompleted: { id: number, completed: boolean, updatedAt: string } };

export type DeleteTodoMutationVariables = Exact<{
  id: number;
}>;


export type DeleteTodoMutation = { deleteTodo: number };

export type CreateTransferMutationVariables = Exact<{
  input: TransferInput;
}>;


export type CreateTransferMutation = { createTransfer: { id: number, fromAccountId: number, toAccountId: number, fromCurrency: string, fromAmount: string, toCurrency: string, toAmount: string, transferDate: string, notes: string | null } };
