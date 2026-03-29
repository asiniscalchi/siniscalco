export type Account = {
  id: number;
  name: string;
  baseCurrency: string;
};

export type Transfer = {
  id: number;
  fromAccountId: number;
  toAccountId: number;
  fromCurrency: string;
  fromAmount: string;
  toCurrency: string;
  toAmount: string;
  transferDate: string;
  notes: string | null;
  createdAt: string;
};
