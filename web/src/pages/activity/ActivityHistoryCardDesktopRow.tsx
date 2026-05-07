import { MoneyText } from "@/lib/money";
import { cn } from "@/lib/utils";

import { ItemLabel } from "@/components/ItemLabel";
import { AssetLabel } from "../assets/AssetLabel";
import { ActivityHistoryCardActions } from "./ActivityHistoryCardActions";
import {
  getActivityTypeClassName,
  getActivityTypeLabel,
  getTransactionTotal,
  trimTrailingZeros,
} from "./ActivityHistoryCard.utils";
import type { Account, ActivityItem, Asset, Transaction } from "./types";

type ActivityHistoryCardDesktopRowProps = {
  item: ActivityItem;
  assetById: Map<number, Asset>;
  accountById: Map<number, Account>;
  hideValues: boolean;
  isLocked: boolean;
  isDeleting: number | null;
  isEditing: boolean;
  onEditClick: (transaction: Transaction) => void;
  onDeleteClick: (transactionId: number) => void;
};

export function ActivityHistoryCardDesktopRow({
  item,
  assetById,
  accountById,
  hideValues,
  isLocked,
  isDeleting,
  isEditing,
  onEditClick,
  onDeleteClick,
}: ActivityHistoryCardDesktopRowProps) {
  return (
    <tr
      className={cn(
        "group transition-colors hover:bg-muted/30",
        isEditing && "bg-muted/50",
      )}
    >
      <td className="py-3 pr-4 whitespace-nowrap tabular-nums">
        {item.date}
      </td>
      <td className="py-3 pr-4">
        {item.kind === "trade" ? (
          <AssetLabel
            name={assetById.get(item.data.assetId)?.name ?? ""}
            symbol={assetById.get(item.data.assetId)?.symbol ?? "Unknown"}
          />
        ) : item.kind === "cash" ? (
          <span className="text-muted-foreground">{item.data.currency}</span>
        ) : (
          <ItemLabel
            primary={accountById.get(item.data.fromAccountId)?.name ?? `Account ${item.data.fromAccountId}`}
            secondary={`→ ${accountById.get(item.data.toAccountId)?.name ?? `Account ${item.data.toAccountId}`}`}
          />
        )}
      </td>
      <td className="py-3 pr-4">
        <span
          className={cn(
            "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide",
            getActivityTypeClassName(item),
          )}
        >
          {getActivityTypeLabel(item)}
        </span>
      </td>
      <td className="py-3 pr-4 text-right font-mono tabular-nums text-muted-foreground">
        {item.kind === "trade" ? trimTrailingZeros(item.data.quantity) : "—"}
      </td>
      <td className="py-3 pr-4 text-right text-muted-foreground">
        {item.kind === "trade" ? (
          <MoneyText
            className="text-right"
            hidden={hideValues}
            includeCurrency={false}
            maximumFractionDigits={8}
            minimumFractionDigits={0}
            value={item.data.unitPrice}
          />
        ) : "—"}
      </td>
      <td className="py-3 pr-4 text-right">
        {item.kind === "trade" ? (
          <MoneyText
            className="text-right"
            hidden={hideValues}
            includeCurrency={false}
            maximumFractionDigits={2}
            minimumFractionDigits={2}
            value={getTransactionTotal(item.data)}
          />
        ) : item.kind === "cash" ? (
          <MoneyText
            className="text-right"
            hidden={hideValues}
            includeCurrency={false}
            maximumFractionDigits={2}
            minimumFractionDigits={2}
            value={Math.abs(Number(item.data.amount))}
          />
        ) : (
          <MoneyText
            className="text-right"
            hidden={hideValues}
            includeCurrency={false}
            maximumFractionDigits={2}
            minimumFractionDigits={2}
            value={item.data.fromAmount}
          />
        )}
      </td>
      <td className="py-3 pr-4 font-mono text-[11px] text-muted-foreground">
        {item.kind === "trade"
          ? item.data.currencyCode
          : item.kind === "cash"
          ? item.data.currency
          : item.data.fromCurrency}
      </td>
      <td
        className="max-w-[150px] truncate py-3 pr-4 italic text-muted-foreground"
        title={item.data.notes || ""}
      >
        {item.data.notes || ""}
      </td>
      {!isLocked ? (
        <td className="py-3 text-right">
          {item.kind === "trade" ? (
            <ActivityHistoryCardActions
              isDeleting={isDeleting}
              isLocked={isLocked}
              onDeleteClick={onDeleteClick}
              onEditClick={onEditClick}
              transaction={item.data}
            />
          ) : null}
        </td>
      ) : null}
    </tr>
  );
}
