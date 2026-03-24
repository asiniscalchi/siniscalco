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

export type ReadyState = {
  account: AccountDetail;
  currencies: string[];
};
