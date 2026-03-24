export type Account = {
  id: number;
  name: string;
  account_type: string;
  base_currency: string;
};

export type Asset = {
  id: number;
  symbol: string;
  name: string;
  asset_type: string;
  isin?: string | null;
};

export type Transaction = {
  id: number;
  account_id: number;
  asset_id: number;
  transaction_type: "BUY" | "SELL";
  trade_date: string;
  quantity: string;
  unit_price: string;
  currency_code: string;
  notes: string | null;
};
