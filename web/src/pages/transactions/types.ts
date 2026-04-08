import type {
  ActivityCashMovementsQuery,
  ActivityTransfersQuery,
  TransactionAccountsQuery,
  TransactionAssetsQuery,
  TransactionsQuery,
} from "@/gql/types";

export type Account = TransactionAccountsQuery["accounts"][number];
export type Asset = TransactionAssetsQuery["assets"][number];
export type Transaction = TransactionsQuery["transactions"][number];
export type CashMovement = ActivityCashMovementsQuery["cashMovements"][number];
export type Transfer = ActivityTransfersQuery["transfers"][number];

export type TradeActivity = { kind: "trade"; date: string; id: string; data: Transaction };
export type CashActivity = { kind: "cash"; date: string; id: string; data: CashMovement };
export type TransferActivity = { kind: "transfer"; date: string; id: string; data: Transfer };
export type ActivityItem = TradeActivity | CashActivity | TransferActivity;

export type ActivityFilter = "all" | "trades" | "cash" | "transfers";
