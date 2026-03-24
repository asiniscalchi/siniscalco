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
