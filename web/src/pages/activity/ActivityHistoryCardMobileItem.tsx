import { Card, CardContent } from "@/components/ui/card";
import { MoneyText } from "@/lib/money";
import { cn } from "@/lib/utils";

import { ItemLabel } from "@/components/ItemLabel";
import { ActivityHistoryCardActions } from "./ActivityHistoryCardActions";
import {
  getActivityTypeClassName,
  getActivityTypeLabel,
  getTransactionTotal,
  trimTrailingZeros,
} from "./ActivityHistoryCard.utils";
import type { Account, ActivityItem, Asset, Transaction } from "./types";

type ActivityHistoryCardMobileItemProps = {
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

export function ActivityHistoryCardMobileItem({
  item,
  assetById,
  accountById,
  hideValues,
  isLocked,
  isDeleting,
  isEditing,
  onEditClick,
  onDeleteClick,
}: ActivityHistoryCardMobileItemProps) {
  return (
    <Card className={cn("bg-background", isEditing && "bg-muted/50")}>
      <CardContent className="space-y-3 p-3">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            {item.kind === "trade" ? (
              <ItemLabel
                primary={assetById.get(item.data.assetId)?.symbol || "Unknown"}
                secondary={assetById.get(item.data.assetId)?.name}
              />
            ) : item.kind === "cash" ? (
              <ItemLabel primary={item.data.currency} secondary="Cash" />
            ) : (
              <ItemLabel
                primary={accountById.get(item.data.fromAccountId)?.name ?? `Account ${item.data.fromAccountId}`}
                secondary={`→ ${accountById.get(item.data.toAccountId)?.name ?? `Account ${item.data.toAccountId}`}`}
              />
            )}
          </div>
          <span
            className={cn(
              "inline-flex shrink-0 items-center rounded-full px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide",
              getActivityTypeClassName(item),
            )}
          >
            {getActivityTypeLabel(item)}
          </span>
        </div>

        {item.kind === "trade" ? (
          <dl className="grid grid-cols-4 gap-x-2 gap-y-2 text-xs">
            <div>
              <dt className="text-[10px] text-muted-foreground">Date</dt>
              <dd className="mt-0.5 font-medium tabular-nums">{item.date}</dd>
            </div>
            <div className="text-center">
              <dt className="text-[10px] text-muted-foreground">Qty</dt>
              <dd className="mt-0.5 font-medium tabular-nums">
                {trimTrailingZeros(item.data.quantity)}
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
                  value={item.data.unitPrice}
                />
              </dd>
            </div>
            <div className="text-right">
              <dt className="text-[10px] text-muted-foreground">Curr</dt>
              <dd className="mt-0.5 font-mono text-[10px] text-muted-foreground">
                {item.data.currencyCode}
              </dd>
            </div>
          </dl>
        ) : (
          <dl className="grid grid-cols-2 gap-x-2 gap-y-2 text-xs">
            <div>
              <dt className="text-[10px] text-muted-foreground">Date</dt>
              <dd className="mt-0.5 font-medium tabular-nums">{item.date}</dd>
            </div>
            <div className="text-right">
              <dt className="text-[10px] text-muted-foreground">Curr</dt>
              <dd className="mt-0.5 font-mono text-[10px] text-muted-foreground">
                {item.kind === "cash" ? item.data.currency : item.data.fromCurrency}
              </dd>
            </div>
          </dl>
        )}

        <div className="flex items-center justify-between gap-3">
          <p className="text-[10px] text-muted-foreground">
            {item.kind === "trade" ? "Total" : "Amount"}
          </p>
          {item.kind === "trade" ? (
            <MoneyText
              className="text-sm"
              hidden={hideValues}
              includeCurrency={false}
              maximumFractionDigits={2}
              minimumFractionDigits={2}
              value={getTransactionTotal(item.data)}
            />
          ) : item.kind === "cash" ? (
            <MoneyText
              className="text-sm"
              hidden={hideValues}
              includeCurrency={false}
              maximumFractionDigits={2}
              minimumFractionDigits={2}
              value={Math.abs(Number(item.data.amount))}
            />
          ) : (
            <MoneyText
              className="text-sm"
              hidden={hideValues}
              includeCurrency={false}
              maximumFractionDigits={2}
              minimumFractionDigits={2}
              value={item.data.fromAmount}
            />
          )}
        </div>

        {item.data.notes ? (
          <div>
            <p className="text-[10px] text-muted-foreground">Notes</p>
            <p className="mt-0.5 truncate text-xs italic text-muted-foreground">
              {item.data.notes}
            </p>
          </div>
        ) : null}

        {!isLocked && item.kind === "trade" ? (
          <ActivityHistoryCardActions
            isDeleting={isDeleting}
            isLocked={isLocked}
            onDeleteClick={onDeleteClick}
            onEditClick={onEditClick}
            transaction={item.data}
          />
        ) : null}
      </CardContent>
    </Card>
  );
}
