import { Card, CardContent, CardHeader } from "@/components/ui/card";

export function AccountDetailLoadingState() {
  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
      <header className="rounded-2xl border bg-background p-6 shadow-sm">
        <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
          Cash Accounts
        </p>
        <h1 className="mt-2 text-3xl font-semibold tracking-tight">
          Account Detail
        </h1>
      </header>
      <div className="grid gap-3">
        {Array.from({ length: 2 }).map((_, index) => (
          <Card key={index} className="border-dashed bg-background/70">
            <CardHeader>
              <div className="h-5 w-40 rounded-full bg-muted" />
              <div className="h-4 w-24 rounded-full bg-muted" />
            </CardHeader>
            <CardContent>
              <div className="h-4 w-32 rounded-full bg-muted" />
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
