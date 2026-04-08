import { PencilIcon, TrashIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";

import type { Transaction } from "./types";

type ActivityHistoryCardActionsProps = {
  isLocked: boolean;
  isDeleting: number | null;
  transaction: Transaction;
  onEditClick: (transaction: Transaction) => void;
  onDeleteClick: (transactionId: number) => void;
};

export function ActivityHistoryCardActions({
  isLocked,
  isDeleting,
  transaction,
  onEditClick,
  onDeleteClick,
}: ActivityHistoryCardActionsProps) {
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
