import { LockIcon, PlusIcon, UnlockIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type AssetPageHeaderProps = {
  isLocked: boolean;
  onToggleLock: () => void;
  onCreateClick: () => void;
};

export function AssetPageHeader({
  isLocked,
  onToggleLock,
  onCreateClick,
}: AssetPageHeaderProps) {
  return (
    <header className="flex flex-col gap-4 rounded-xl border bg-background px-6 py-5 shadow-sm sm:flex-row sm:items-center sm:justify-between">
      <div className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Assets</h1>
        <p className="max-w-2xl text-sm text-muted-foreground">
          Manage the assets you use in transactions.
        </p>
      </div>
      <div className="flex items-center justify-end gap-2">
        <Button
          aria-label={isLocked ? "Unlock edit mode" : "Lock edit mode"}
          className={cn(
            "size-9 rounded-full transition-colors",
            !isLocked &&
              "bg-amber-100 text-amber-900 hover:bg-amber-200 dark:bg-amber-900/30 dark:text-amber-400 dark:hover:bg-amber-900/50",
          )}
          onClick={onToggleLock}
          size="icon"
          type="button"
          variant="ghost"
        >
          {isLocked ? <LockIcon /> : <UnlockIcon />}
        </Button>
        <Button
          aria-label="Add Asset"
          onClick={onCreateClick}
          size="icon-lg"
          title="Add Asset"
        >
          <PlusIcon />
        </Button>
      </div>
    </header>
  );
}
