import type { ActivityItem, Transaction } from "./types";

export function trimTrailingZeros(value: string) {
  return value.replace(/\.?0+$/, "");
}

export function getTransactionTotal(transaction: Transaction) {
  return Number(transaction.quantity) * Number(transaction.unitPrice);
}

export function getTransactionTypeClassName(
  transactionType: Transaction["transactionType"],
) {
  return transactionType === "BUY"
    ? "border border-emerald-200 bg-emerald-50 text-emerald-700"
    : "border border-amber-200 bg-amber-50 text-amber-700";
}

export function getActivityTypeLabel(item: ActivityItem): string {
  if (item.kind === "trade") return item.data.transactionType;
  if (item.kind === "cash") return Number(item.data.amount) >= 0 ? "DEPOSIT" : "WITHDRAWAL";
  return "TRANSFER";
}

export function getActivityTypeClassName(item: ActivityItem): string {
  if (item.kind === "trade") return getTransactionTypeClassName(item.data.transactionType);
  if (item.kind === "cash") {
    return Number(item.data.amount) >= 0
      ? "border border-blue-200 bg-blue-50 text-blue-700"
      : "border border-rose-200 bg-rose-50 text-rose-700";
  }
  return "border border-violet-200 bg-violet-50 text-violet-700";
}
