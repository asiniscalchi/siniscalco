import type { TransferAccountsQuery } from "@/gql/types";

export type Account = TransferAccountsQuery["accounts"][number];
