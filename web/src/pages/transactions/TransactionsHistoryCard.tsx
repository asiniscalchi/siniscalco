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

  function renderActions(transaction: Transaction) {
    return (
      <div className="flex justify-end gap-1">
        <Button
          disabled={isLocked || isDeleting !== null}
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
          disabled={isLocked || isDeleting !== null}
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
    );
  }

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
          <>
            <div className="space-y-2 sm:hidden">
              {transactions.map((transaction) => {
                const asset = assets.find((item) => item.id === transaction.asset_id);

                return (
                  <Card
                    className={cn(
                      "bg-background",
                      editingTransactionId === transaction.id && "bg-muted/50",
                    )}
                    key={transaction.id}
                  >
                    <CardContent className="space-y-3 p-3">
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-sm font-semibold leading-tight">
                            {asset?.symbol || "Unknown"}
                          </p>
                          <p className="truncate text-[11px] leading-tight text-muted-foreground">
                            {asset?.name}
                          </p>
                        </div>
                        <span
                          className={cn(
                            "inline-flex shrink-0 items-center rounded-full px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide",
                            transaction.transaction_type === "BUY"
                              ? "border border-emerald-200 bg-emerald-50 text-emerald-700"
                              : "border border-amber-200 bg-amber-50 text-amber-700",
                          )}
                        >
                          {transaction.transaction_type}
                        </span>
                      </div>

                      <dl className="grid grid-cols-4 gap-x-2 gap-y-2 text-xs">
                        <div>
                          <dt className="text-[10px] text-muted-foreground">Date</dt>
                          <dd className="mt-0.5 font-medium tabular-nums">
                            {transaction.trade_date}
                          </dd>
                        </div>
                        <div>
                          <dt className="text-[10px] text-muted-foreground">Qty</dt>
                          <dd className="mt-0.5 text-right font-medium tabular-nums">
                            {transaction.quantity}
                          </dd>
                        </div>
                        <div>
                          <dt className="text-[10px] text-muted-foreground">Price</dt>
                          <dd className="mt-0.5">
                            <MoneyText
                              className="text-sm"
                              hidden={hideValues}
                              includeCurrency={false}
                              value={transaction.unit_price}
                            />
                          </dd>
                        </div>
                        <div>
                          <dt className="text-[10px] text-muted-foreground">Curr</dt>
                          <dd className="mt-0.5 font-mono text-[10px] text-muted-foreground">
                            {transaction.currency_code}
                          </dd>
                        </div>
                      </dl>

                      <div>
                        <p className="text-[10px] text-muted-foreground">Notes</p>
                        <p className="mt-0.5 truncate text-xs italic text-muted-foreground">
                          {transaction.notes || "—"}
                        </p>
                      </div>

                      {renderActions(transaction)}
                    </CardContent>
                  </Card>
                );
              })}
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
                    <th className="w-[90px] pb-3 text-right">Actions</th>
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
                          <div className="flex flex-col">
                            <span className="font-bold">{asset?.symbol || "Unknown"}</span>
                            <span className="max-w-[120px] truncate text-[10px] text-muted-foreground">
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
                        <td className="py-3 pr-4 text-right font-mono tabular-nums">
                          {transaction.quantity}
                        </td>
                        <td className="py-3 pr-4 text-right">
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
                          className="max-w-[150px] truncate py-3 pr-4 italic text-muted-foreground"
                          title={transaction.notes || ""}
                        >
                          {transaction.notes}
                        </td>
                        <td className="py-3 text-right">{renderActions(transaction)}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
