import { LockIcon, PlusIcon, UnlockIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import type { Account } from "./types";

type TransactionsPageHeaderProps = {
  accounts: Account[];
  selectedAccountId: string;
  isLocked: boolean;
  onAccountChange: (accountId: string) => void;
  onToggleLock: () => void;
  onCreateClick: () => void;
};

export function TransactionsPageHeader({
  accounts,
  selectedAccountId,
  isLocked,
  onAccountChange,
  onToggleLock,
  onCreateClick,
}: TransactionsPageHeaderProps) {
  return (
    <header className="flex flex-col gap-4 rounded-xl border bg-background px-6 py-5 shadow-sm sm:flex-row sm:items-center sm:justify-between">
      <div className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Transactions</h1>
        <p className="max-w-2xl text-sm text-muted-foreground">
          Manage your asset transactions.
        </p>
      </div>
      <div className="flex w-full flex-wrap items-center gap-4 sm:w-auto sm:flex-nowrap">
        <div className="flex min-w-0 items-center gap-3">
          <label
            className="text-sm font-medium text-muted-foreground"
            htmlFor="account-selector"
          >
            Account:
          </label>
          <select
            className="rounded-md border bg-background px-3 py-1.5 text-sm shadow-sm transition-colors focus:outline-hidden focus:ring-1 focus:ring-ring"
            id="account-selector"
            onChange={(event) => onAccountChange(event.target.value)}
            value={selectedAccountId}
          >
            <option value="">All Accounts</option>
            {accounts.map((account) => (
              <option key={account.id} value={String(account.id)}>
                {account.name}
              </option>
            ))}
          </select>
        </div>
        <div className="ml-auto flex items-center justify-end gap-2">
          <Button
            aria-label={isLocked ? "Unlock edit mode" : "Lock edit mode"}
            className={cn(
              "size-9 rounded-full transition-colors",
              !isLocked &&
                "bg-amber-100 text-amber-900 hover:bg-amber-200 dark:bg-amber-900/30 dark:text-amber-400 dark:hover:bg-amber-900/50",
            )}
            onClick={onToggleLock}
            size="icon"
            type="button"
            variant="ghost"
          >
            {isLocked ? <LockIcon /> : <UnlockIcon />}
          </Button>
          <Button
            aria-label="Add Transaction"
            disabled={!selectedAccountId}
            onClick={onCreateClick}
            size="icon-lg"
            title="Add Transaction"
          >
            <PlusIcon />
          </Button>
        </div>
      </div>
    </header>
  );
}
