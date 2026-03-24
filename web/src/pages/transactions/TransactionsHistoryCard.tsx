import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

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
}: TransactionsHistoryCardProps) {
  const selectedAccountName = selectedAccountId
    ? accounts.find((account) => String(account.id) === selectedAccountId)?.name
    : null;
  const assetById = new Map(assets.map((asset) => [asset.id, asset]));

  return (
    <Card className="min-w-0 bg-background">
      <CardHeader>
        <CardTitle>Transaction History</CardTitle>
        <CardDescription>
          {selectedAccountId
            ? `Recent transactions for ${selectedAccountName || "selected account"}.`
            : "Showing all recorded transactions."}
        </CardDescription>
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
