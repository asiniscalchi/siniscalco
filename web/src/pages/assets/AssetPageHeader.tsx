type AssetPageHeaderProps = {
  isLocked: boolean;
};

export function AssetPageHeader({
  isLocked,
}: AssetPageHeaderProps) {
  return (
    <header className="flex flex-col gap-4 rounded-xl border bg-background px-6 py-5 shadow-sm sm:flex-row sm:items-center sm:justify-between">
      <div className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Assets</h1>
        <p className="max-w-2xl text-sm text-muted-foreground">
          Manage the assets you use in transactions.
        </p>
      </div>
    </header>
  );
}
