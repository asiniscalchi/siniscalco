export type Account = {
  id: number;
  name: string;
  accountType: string;
  baseCurrency: string;
};

export type Asset = {
  id: number;
  symbol: string;
  name: string;
  assetType: string;
  isin?: string | null;
};

export type Transaction = {
  id: number;
  accountId: number;
  assetId: number;
  transactionType: "BUY" | "SELL";
  tradeDate: string;
  quantity: string;
  unitPrice: string;
  currencyCode: string;
  notes: string | null;
};
