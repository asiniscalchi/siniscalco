import type { Transaction } from "./types";

export function trimTrailingZeros(value: string) {
  return value.replace(/\.?0+$/, "");
}

export function getTransactionTypeClassName(
  transactionType: Transaction["transaction_type"],
) {
  return transactionType === "BUY"
    ? "border border-emerald-200 bg-emerald-50 text-emerald-700"
    : "border border-amber-200 bg-amber-50 text-amber-700";
}
