import type { AccountQuery } from "@/gql/types";

export type AccountDetail = AccountQuery["account"];
export type AccountBalance = AccountDetail["balances"][number];
