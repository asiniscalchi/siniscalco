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

type TransactionsHistoryCardDesktopRowProps = {
  asset: Asset | undefined;
  transaction: Transaction;
  hideValues: boolean;
  isLocked: boolean;
  isDeleting: number | null;
  isEditing: boolean;
  onEditClick: (transaction: Transaction) => void;
  onDeleteClick: (transactionId: number) => void;
};

export function TransactionsHistoryCardDesktopRow({
  asset,
  transaction,
  hideValues,
  isLocked,
  isDeleting,
  isEditing,
  onEditClick,
  onDeleteClick,
}: TransactionsHistoryCardDesktopRowProps) {
  const total = getTransactionTotal(transaction);

  return (
    <tr
      className={cn(
        "group transition-colors hover:bg-muted/30",
        isEditing && "bg-muted/50",
      )}
    >
      <td className="py-3 pr-4 whitespace-nowrap tabular-nums">
        {transaction.tradeDate}
      </td>
      <td className="py-3 pr-4">
        <ItemLabel primary={asset?.symbol || "Unknown"} secondary={asset?.name} />
      </td>
      <td className="py-3 pr-4">
        <span
          className={cn(
            "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide",
            getTransactionTypeClassName(transaction.transactionType),
          )}
        >
          {transaction.transactionType}
        </span>
      </td>
      <td className="py-3 pr-4 text-right font-mono tabular-nums">
        {trimTrailingZeros(transaction.quantity)}
      </td>
      <td className="py-3 pr-4 text-right">
        <MoneyText
          className="text-right"
          hidden={hideValues}
          includeCurrency={false}
          maximumFractionDigits={8}
          minimumFractionDigits={0}
          value={transaction.unitPrice}
        />
      </td>
      <td className="py-3 pr-4 text-right">
        <MoneyText
          className="text-right"
          hidden={hideValues}
          includeCurrency={false}
          maximumFractionDigits={2}
          minimumFractionDigits={2}
          value={total}
        />
      </td>
      <td className="py-3 pr-4 font-mono text-[11px] text-muted-foreground">
        {transaction.currencyCode}
      </td>
      <td
        className="max-w-[150px] truncate py-3 pr-4 italic text-muted-foreground"
        title={transaction.notes || ""}
      >
        {transaction.notes || ""}
      </td>
      {!isLocked ? (
        <td className="py-3 text-right">
          <TransactionsHistoryCardActions
            isDeleting={isDeleting}
            isLocked={isLocked}
            onDeleteClick={onDeleteClick}
            onEditClick={onEditClick}
            transaction={transaction}
          />
        </td>
      ) : null}
    </tr>
  );
}
