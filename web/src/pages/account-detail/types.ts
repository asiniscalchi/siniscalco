export type AccountBalance = {
  currency: string;
  amount: string;
  updatedAt: string;
};

export type AccountDetail = {
  id: number;
  name: string;
  accountType: string;
  baseCurrency: string;
  summaryStatus: "OK" | "CONVERSION_UNAVAILABLE";
  cashTotalAmount: string | null;
  assetTotalAmount: string | null;
  totalAmount: string | null;
  totalCurrency: string | null;
  createdAt: string;
  balances: AccountBalance[];
};

export type AccountAsset = {
  assetId: number;
  symbol: string;
  name: string;
  assetType: string;
  quantity: string;
  value: string | null;
};

export type ReadyState = {
  account: AccountDetail;
  currencies: string[];
  assets: AccountAsset[];
};
