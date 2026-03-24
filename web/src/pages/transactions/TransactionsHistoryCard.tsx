import { PencilIcon, TrashIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { MoneyText } from "@/lib/money";
import { cn } from "@/lib/utils";

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
          <div className="py-12 text-center text-sm text-muted-foreground">
            No transactions recorded.
          </div>
        ) : (
          <div className="min-w-0 w-full overflow-x-auto overflow-y-hidden">
            <table className="w-full min-w-[800px] text-sm">
              <thead>
                <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                  <th className="pb-3 pr-4 whitespace-nowrap">Date</th>
                  <th className="pb-3 pr-4">Asset</th>
                  <th className="pb-3 pr-4 whitespace-nowrap">Type</th>
                  <th className="pb-3 pr-4 text-right whitespace-nowrap">Quantity</th>
                  <th className="pb-3 pr-4 text-right whitespace-nowrap">Price</th>
                  <th className="pb-3 pr-4 whitespace-nowrap">Curr</th>
                  <th className="pb-3 pr-4">Notes</th>
                  {!isLocked && <th className="pb-3 text-right whitespace-nowrap">Actions</th>}
                </tr>
              </thead>
              <tbody className="divide-y">
                {transactions.map((transaction) => {
                  const asset = assets.find((item) => item.id === transaction.asset_id);

                  return (
                    <tr
                      className={cn(
                        "group transition-colors hover:bg-muted/30",
                        editingTransactionId === transaction.id && "bg-muted/50",
                      )}
                      key={transaction.id}
                    >
                      <td className="py-3 pr-4 whitespace-nowrap tabular-nums">
                        {transaction.trade_date}
                      </td>
                      <td className="py-3 pr-4">
                        <div className="flex flex-col min-w-[120px]">
                          <span className="font-bold">{asset?.symbol || "Unknown"}</span>
                          <span className="truncate text-[10px] text-muted-foreground">
                            {asset?.name}
                          </span>
                        </div>
                      </td>
                      <td className="py-3 pr-4">
                        <span
                          className={cn(
                            "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide",
                            transaction.transaction_type === "BUY"
                              ? "border border-emerald-200 bg-emerald-50 text-emerald-700"
                              : "border border-amber-200 bg-amber-50 text-amber-700",
                          )}
                        >
                          {transaction.transaction_type}
                        </span>
                      </td>
                      <td className="py-3 pr-4 text-right font-mono tabular-nums whitespace-nowrap">
                        {transaction.quantity}
                      </td>
                      <td className="py-3 pr-4 text-right whitespace-nowrap">
                        <MoneyText
                          className="text-right"
                          hidden={hideValues}
                          includeCurrency={false}
                          value={transaction.unit_price}
                        />
                      </td>
                      <td className="py-3 pr-4 font-mono text-[11px] text-muted-foreground">
                        {transaction.currency_code}
                      </td>
                      <td
                        className="max-w-[200px] truncate py-3 pr-4 italic text-muted-foreground"
                        title={transaction.notes || ""}
                      >
                        {transaction.notes}
                      </td>
                      {!isLocked && (
                        <td className="py-3 text-right">
                          <div className="flex justify-end gap-1">
                            <Button
                              disabled={isDeleting !== null}
                              onClick={() => onEditClick(transaction)}
                              size="icon"
                              title="Edit transaction"
                              type="button"
                              variant="ghost"
                            >
                              <PencilIcon />
                              <span className="sr-only">Edit</span>
                            </Button>
                            <Button
                              className="text-destructive hover:bg-destructive/10"
                              disabled={isDeleting !== null}
                              onClick={() => onDeleteClick(transaction.id)}
                              size="icon"
                              title="Delete transaction"
                              type="button"
                              variant="ghost"
                            >
                              {isDeleting === transaction.id ? (
                                <div className="size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
                              ) : (
                                <TrashIcon />
                              )}
                              <span className="sr-only">Delete</span>
                            </Button>
                          </div>
                        </td>
                      )}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
