export type AccountBalance = {
  currency: string;
  amount: string;
  updated_at: string;
};

export type AccountDetail = {
  id: number;
  name: string;
  account_type: string;
  base_currency: string;
  summary_status: "ok" | "conversion_unavailable";
  cash_total_amount: string | null;
  asset_total_amount: string | null;
  total_amount: string | null;
  total_currency: string | null;
  created_at: string;
  balances: AccountBalance[];
};

export type AccountAsset = {
  asset_id: number;
  symbol: string;
  name: string;
  asset_type: string;
  quantity: string;
};

export type ReadyState = {
  account: AccountDetail;
  currencies: string[];
  assets: AccountAsset[];
};
