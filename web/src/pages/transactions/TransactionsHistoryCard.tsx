import { LockIcon, PlusIcon, UnlockIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { cn } from "@/lib/utils";

import { TransactionsHistoryCardDesktopRow } from "./TransactionsHistoryCardDesktopRow";
import { TransactionsHistoryCardEmptyState } from "./TransactionsHistoryCardEmptyState";
import { TransactionsHistoryCardMobileItem } from "./TransactionsHistoryCardMobileItem";
import type { Account, Asset, Transaction } from "./types";

type TransactionsHistoryCardProps = {
  accounts: Account[];
  assets: Asset[];
  transactions: Transaction[];
  selectedAccountId: string;
  hideValues: boolean;
  isLocked: boolean;
  editingTransactionId: number | null;
  isDeleting: number | null;
  onEditClick: (transaction: Transaction) => void;
  onDeleteClick: (transactionId: number) => void;
  onAccountChange: (accountId: string) => void;
  onToggleLock: () => void;
  onCreateClick: () => void;
};

export function TransactionsHistoryCard({
  accounts,
  assets,
  transactions,
  selectedAccountId,
  hideValues,
  isLocked,
  editingTransactionId,
  isDeleting,
  onEditClick,
  onDeleteClick,
  onAccountChange,
  onToggleLock,
  onCreateClick,
}: TransactionsHistoryCardProps) {
  const assetById = new Map(assets.map((asset) => [asset.id, asset]));

  return (
    <Card className="min-w-0 bg-background">
      <CardHeader className="pb-2">
        <div className="flex items-center gap-2">
          <h1 className="flex-1 text-2xl font-semibold tracking-tight">Transactions</h1>
          <label className="sr-only" htmlFor="account-selector">Account:</label>
          <select
            className="hidden rounded-md border bg-background px-3 py-1.5 text-sm shadow-sm transition-colors focus:outline-hidden focus:ring-1 focus:ring-ring sm:block"
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
          <Button
            aria-label={isLocked ? "Unlock edit mode" : "Lock edit mode"}
            className={cn(
              "size-9 rounded-full transition-colors",
              !isLocked &&
                "bg-amber-100 text-amber-900 hover:bg-amber-200 dark:bg-amber-900/30 dark:text-amber-400 dark:hover:bg-amber-900/50",
            )}
            onClick={onToggleLock}
            size="icon"
            title={isLocked ? "Unlock edit mode" : "Lock edit mode"}
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
        <div className="flex justify-end sm:hidden">
          <select
            aria-label="Account"
            className="rounded-md border bg-background px-3 py-1.5 text-sm shadow-sm transition-colors focus:outline-hidden focus:ring-1 focus:ring-ring"
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
      </CardHeader>
      <CardContent className="min-w-0 pt-4">
        {transactions.length === 0 ? (
          <TransactionsHistoryCardEmptyState />
        ) : (
          <>
            <div className="space-y-2 sm:hidden">
              {transactions.map((transaction) => (
                <TransactionsHistoryCardMobileItem
                  asset={assetById.get(transaction.asset_id)}
                  hideValues={hideValues}
                  isDeleting={isDeleting}
                  isEditing={editingTransactionId === transaction.id}
                  isLocked={isLocked}
                  key={transaction.id}
                  onDeleteClick={onDeleteClick}
                  onEditClick={onEditClick}
                  transaction={transaction}
                />
              ))}
            </div>

            <div className="hidden w-full overflow-x-auto sm:block">
              <table className="w-full table-fixed text-sm">
                <thead>
                  <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                    <th className="w-[100px] pb-3 pr-4">Date</th>
                    <th className="pb-3 pr-4">Asset</th>
                    <th className="w-[80px] pb-3 pr-4">Type</th>
                    <th className="w-[100px] pb-3 pr-4 text-right">Quantity</th>
                    <th className="w-[100px] pb-3 pr-4 text-right">Price</th>
                    <th className="w-[60px] pb-3 pr-4">Curr</th>
                    <th className="pb-3 pr-4">Notes</th>
                    {!isLocked ? <th className="w-[90px] pb-3 text-right">Actions</th> : null}
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {transactions.map((transaction) => (
                    <TransactionsHistoryCardDesktopRow
                      asset={assetById.get(transaction.asset_id)}
                      hideValues={hideValues}
                      isDeleting={isDeleting}
                      isEditing={editingTransactionId === transaction.id}
                      isLocked={isLocked}
                      key={transaction.id}
                      onDeleteClick={onDeleteClick}
                      onEditClick={onEditClick}
                      transaction={transaction}
                    />
                  ))}
                </tbody>
              </table>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
