import type {
  TransactionAccountsQuery,
  TransactionAssetsQuery,
  TransactionsQuery,
} from "@/gql/types";

export type Account = TransactionAccountsQuery["accounts"][number];
export type Asset = TransactionAssetsQuery["assets"][number];
export type Transaction = TransactionsQuery["transactions"][number];
