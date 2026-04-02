export function getTodayDate(): string {
  return new Date().toISOString().split("T")[0];
}

export function getAccountCurrency(
  accounts: { id: number; baseCurrency: string }[],
  accountId: string,
): string {
  return accounts.find((a) => String(a.id) === accountId)?.baseCurrency ?? "";
}
