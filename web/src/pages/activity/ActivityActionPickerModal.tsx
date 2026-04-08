import { ModalDialog } from "@/components/ModalDialog";
import { Button } from "@/components/ui/button";
import { useBodyScrollLock } from "@/lib/use-body-scroll-lock";

type ActivityCreateAction = "trade" | "deposit" | "withdraw" | "transfer";

type ActivityActionPickerModalProps = {
  canTransfer: boolean;
  open: boolean;
  selectedAccountName: string | null;
  onClose: () => void;
  onSelect: (action: ActivityCreateAction) => void;
};

const ACTIONS: Array<{ value: ActivityCreateAction; label: string; description: string }> = [
  {
    value: "trade",
    label: "Trade",
    description: "Record a buy or sell transaction for the selected account.",
  },
  {
    value: "deposit",
    label: "Deposit",
    description: "Add cash to the selected account.",
  },
  {
    value: "withdraw",
    label: "Withdraw",
    description: "Remove cash from the selected account.",
  },
  {
    value: "transfer",
    label: "Transfer",
    description: "Move funds between accounts.",
  },
];

export function ActivityActionPickerModal({
  canTransfer,
  open,
  selectedAccountName,
  onClose,
  onSelect,
}: ActivityActionPickerModalProps) {
  useBodyScrollLock(open);

  if (!open) return null;

  return (
    <ModalDialog
      description={`Choose what to record for ${selectedAccountName ?? "the selected account"}.`}
      title="Record Activity"
    >
      <div className="flex flex-1 flex-col overflow-hidden">
        <div className="grid flex-1 min-h-0 gap-3 overflow-y-auto px-6 py-6">
          {ACTIONS.map((action) => (
            <button
              className="rounded-lg border bg-background px-4 py-4 text-left transition-colors enabled:hover:bg-muted/50 disabled:cursor-not-allowed disabled:opacity-50"
              disabled={action.value === "transfer" && !canTransfer}
              key={action.value}
              onClick={() => onSelect(action.value)}
              type="button"
            >
              <div className="font-medium">{action.label}</div>
              <div className="mt-1 text-sm text-muted-foreground">{action.description}</div>
            </button>
          ))}
        </div>
        <footer className="flex flex-none justify-end rounded-b-xl border-t bg-muted/30 px-6 py-4">
          <Button onClick={onClose} type="button" variant="outline">
            Cancel
          </Button>
        </footer>
      </div>
    </ModalDialog>
  );
}

export type { ActivityCreateAction };
