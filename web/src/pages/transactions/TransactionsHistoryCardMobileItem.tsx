import { Card, CardContent } from "@/components/ui/card";
import { MoneyText } from "@/lib/money";
import { cn } from "@/lib/utils";

import { ItemLabel } from "@/components/ItemLabel";
import { TransactionsHistoryCardActions } from "./TransactionsHistoryCardActions";
import {
  getTransactionTotal,
  getTransactionTypeClassName,
  trimTrailingZeros,
} from "./TransactionsHistoryCard.utils";
import type { Asset, Transaction } from "./types";

type TransactionsHistoryCardMobileItemProps = {
  asset: Asset | undefined;
  transaction: Transaction;
  hideValues: boolean;
  isLocked: boolean;
  isDeleting: number | null;
  isEditing: boolean;
  onEditClick: (transaction: Transaction) => void;
  onDeleteClick: (transactionId: number) => void;
};

export function TransactionsHistoryCardMobileItem({
  asset,
  transaction,
  hideValues,
  isLocked,
  isDeleting,
  isEditing,
  onEditClick,
  onDeleteClick,
}: TransactionsHistoryCardMobileItemProps) {
  const total = getTransactionTotal(transaction);

  return (
    <Card className={cn("bg-background", isEditing && "bg-muted/50")}>
      <CardContent className="space-y-3 p-3">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <ItemLabel primary={asset?.symbol || "Unknown"} secondary={asset?.name} />
          </div>
          <span
            className={cn(
              "inline-flex shrink-0 items-center rounded-full px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide",
              getTransactionTypeClassName(transaction.transactionType),
            )}
          >
            {transaction.transactionType}
          </span>
        </div>

        <dl className="grid grid-cols-4 gap-x-2 gap-y-2 text-xs">
          <div>
            <dt className="text-[10px] text-muted-foreground">Date</dt>
            <dd className="mt-0.5 font-medium tabular-nums">
              {transaction.tradeDate}
            </dd>
          </div>
          <div className="text-center">
            <dt className="text-[10px] text-muted-foreground">Qty</dt>
            <dd className="mt-0.5 font-medium tabular-nums">
              {trimTrailingZeros(transaction.quantity)}
            </dd>
          </div>
          <div className="text-center">
            <dt className="text-[10px] text-muted-foreground">Price</dt>
            <dd className="mt-0.5">
              <MoneyText
                className="text-sm"
                hidden={hideValues}
                includeCurrency={false}
                maximumFractionDigits={8}
                minimumFractionDigits={0}
                value={transaction.unitPrice}
              />
            </dd>
          </div>
          <div className="text-right">
            <dt className="text-[10px] text-muted-foreground">Curr</dt>
            <dd className="mt-0.5 font-mono text-[10px] text-muted-foreground">
              {transaction.currencyCode}
            </dd>
          </div>
        </dl>

        <div className="flex items-center justify-between gap-3">
          <p className="text-[10px] text-muted-foreground">Total</p>
          <MoneyText
            className="text-sm"
            hidden={hideValues}
            includeCurrency={false}
            maximumFractionDigits={2}
            minimumFractionDigits={2}
            value={total}
          />
        </div>

        {transaction.notes ? (
          <div>
            <p className="text-[10px] text-muted-foreground">Notes</p>
            <p className="mt-0.5 truncate text-xs italic text-muted-foreground">
              {transaction.notes}
            </p>
          </div>
        ) : null}

        {!isLocked ? (
          <TransactionsHistoryCardActions
            isDeleting={isDeleting}
            isLocked={isLocked}
            onDeleteClick={onDeleteClick}
            onEditClick={onEditClick}
            transaction={transaction}
          />
        ) : null}
      </CardContent>
    </Card>
  );
}
