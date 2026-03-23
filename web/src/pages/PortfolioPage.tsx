import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { buttonVariants } from "@/components/ui/button-variants";
import {
  getPortfolioApiUrl,
  type PortfolioSummaryResponse,
} from "@/lib/api";
import { cn } from "@/lib/utils";

type PortfolioSummary = PortfolioSummaryResponse;

export function PortfolioPage() {
  const [requestState, setRequestState] = useState<
    | { status: "loading" }
    | { status: "error" }
    | { status: "ready"; summary: PortfolioSummary }
  >({ status: "loading" });
  const [retryToken, setRetryToken] = useState(0);

  useEffect(() => {
    let cancelled = false;

    async function loadPortfolio() {
      setRequestState({ status: "loading" });

      try {
        const response = await fetch(getPortfolioApiUrl());

        if (!response.ok) {
          throw new Error(`portfolio request failed with status ${response.status}`);
        }

        const summary =
          (await response.json()) as PortfolioSummaryResponse;

        if (!cancelled) {
          setRequestState({ status: "ready", summary });
        }
      } catch {
        if (!cancelled) {
          setRequestState({ status: "error" });
        }
      }
    }

    void loadPortfolio();

    return () => {
      cancelled = true;
    };
  }, [retryToken]);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-8">
      {requestState.status === "loading" ? (
        <>
          <header className="px-1">
            <h1 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
              Portfolio
            </h1>
          </header>
          <PortfolioLoadingState />
        </>
      ) : null}
      {requestState.status === "error" ? (
        <PortfolioErrorState onRetry={() => setRetryToken((value) => value + 1)} />
      ) : null}
      {requestState.status === "ready" ? (
        <PortfolioReadyState summary={requestState.summary} />
      ) : null}
    </div>
  );
}

function PortfolioLoadingState() {
  return (
    <div className="grid gap-4 md:grid-cols-2">
      {Array.from({ length: 4 }).map((_, index) => (
        <Card key={index} className="border-dashed bg-background/70">
          <CardHeader>
            <div className="h-5 w-32 rounded-full bg-muted" />
            <div className="h-4 w-48 rounded-full bg-muted" />
          </CardHeader>
        </Card>
      ))}
    </div>
  );
}

function PortfolioErrorState({ onRetry }: { onRetry: () => void }) {
  return (
    <Card className="border-destructive/30 bg-background">
      <CardHeader>
        <CardTitle>Could not load portfolio</CardTitle>
        <CardDescription>
          The portfolio overview request failed. Try again to reload the page
          data.
        </CardDescription>
      </CardHeader>
      <CardFooter className="justify-end gap-3">
        <Link className={cn(buttonVariants({ variant: "outline" }))} to="/accounts">
          View accounts
        </Link>
        <Button onClick={onRetry} type="button">
          Retry
        </Button>
      </CardFooter>
    </Card>
  );
}

function PortfolioReadyState({ summary }: { summary: PortfolioSummary }) {
  const hasCashData = summary.cash_by_currency.length > 0;

  if (!hasCashData) {
    return (
      <>
        <header className="px-1">
          <h1 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
            Portfolio
          </h1>
        </header>
        <PortfolioEmptyState />
      </>
    );
  }

  const totalValue = summary.total_value_amount ? Number(summary.total_value_amount) : null;

  return (
    <div className="flex flex-col gap-8">
      {/* Portfolio Summary Area */}
      <section className="space-y-4">
        <div className="flex flex-col gap-1.5 px-1">
          <h1 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
            Portfolio
          </h1>
          <div className="flex items-baseline gap-4">
            {summary.total_value_status === "ok" && summary.total_value_amount ? (
              <span className="text-4xl font-bold tracking-tight">
                {formatDisplayAmount(summary.total_value_amount, summary.display_currency)}
              </span>
            ) : (
              <span className="text-2xl font-semibold text-muted-foreground">
                Conversion unavailable
              </span>
            )}
            <span className="text-sm text-muted-foreground font-medium">Total Cash Value</span>
          </div>
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            {summary.total_value_status === "conversion_unavailable" && (
              <span className="text-destructive font-medium">Conversion data unavailable</span>
            )}
            {summary.total_value_status === "ok" && (
              <span>Converted to {summary.display_currency}</span>
            )}
            <span>•</span>
            <span>
              Last FX update:{" "}
              {summary.fx_last_updated ? formatTimestamp(summary.fx_last_updated) : "unavailable"}
            </span>
            {summary.fx_refresh_status === "unavailable" && (
              <>
                <span>•</span>
                <span className="text-destructive font-medium">
                  FX refresh unavailable
                </span>
              </>
            )}
          </div>
          {summary.fx_refresh_status === "unavailable" && summary.fx_refresh_error ? (
            <div className="text-xs text-destructive/90">
              {summary.fx_refresh_error}
            </div>
          ) : null}
        </div>
      </section>

      <div className="grid gap-8 lg:grid-cols-[1fr_350px]">
        {/* Account breakdown */}
        <section className="space-y-4">
          <h2 className="text-lg font-semibold tracking-tight px-1">
            Cash By Account
          </h2>
          <Card className="bg-background">
            <CardContent className="pt-6">
              <div className="space-y-6">
                {[...summary.account_totals]
                  .sort((a, b) => {
                    const valA = a.total_amount ? Number(a.total_amount) : 0;
                    const valB = b.total_amount ? Number(b.total_amount) : 0;
                    return valB - valA;
                  })
                  .map((account) => {
                    const accountValue = account.total_amount
                      ? Number(account.total_amount)
                      : 0;
                    const percentage =
                      totalValue && account.summary_status === "ok"
                        ? (accountValue / totalValue) * 100
                        : 0;

                    return (
                      <div key={account.id} className="space-y-1">
                        <div className="flex items-end justify-between text-sm">
                          <div className="flex flex-col">
                            <span className="font-medium">{account.name}</span>
                            <span className="text-xs text-muted-foreground opacity-80">
                              {account.account_type}
                            </span>
                          </div>
                          <div className="text-right">
                            {account.summary_status === "ok" &&
                            account.total_amount ? (
                              <span className="font-mono text-xs text-muted-foreground">
                                {formatDisplayAmount(
                                  account.total_amount,
                                  account.total_currency,
                                )}
                              </span>
                            ) : (
                              <span className="text-xs font-medium text-muted-foreground">
                                Conversion unavailable
                              </span>
                            )}
                          </div>
                        </div>
                        <div className="h-2 w-full bg-muted rounded-full overflow-hidden">
                          <div
                            className="h-full bg-primary transition-all duration-500"
                            style={{ width: `${percentage}%` }}
                          />
                        </div>
                      </div>
                    );
                  })}
              </div>
            </CardContent>
            {summary.account_totals.some(
              (a) => a.summary_status !== "ok",
            ) && (
              <CardFooter className="pt-0">
                <p className="text-xs text-muted-foreground italic">
                  * Some accounts are hidden or incomplete due to missing
                  conversion rates.
                </p>
              </CardFooter>
            )}
          </Card>
        </section>

        {/* Cash by currency */}
        <section className="space-y-4">
          <h2 className="text-lg font-semibold tracking-tight px-1">Cash By Currency</h2>
          <Card className="bg-background">
            <CardContent className="pt-6 pb-6">
              <div className="space-y-6">
                {summary.cash_by_currency.map((balance) => {
                  const balanceValue = balance.converted_amount ? Number(balance.converted_amount) : null;
                  const percentage =
                    totalValue && balanceValue ? (balanceValue / totalValue) * 100 : 0;

                  return (
                    <div key={balance.currency} className="space-y-1">
                      <div className="flex items-end justify-between text-sm">
                        <span className="font-medium">{balance.currency}</span>
                        <span className="font-mono text-xs text-muted-foreground">
                          {formatOriginalAmount(balance.amount)} {balance.currency}
                        </span>
                      </div>
                      <div className="h-2 w-full bg-muted rounded-full overflow-hidden">
                        <div
                          className="h-full bg-primary transition-all duration-500"
                          style={{ width: `${percentage}%` }}
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            </CardContent>
          </Card>
        </section>
      </div>
    </div>
  );
}

function PortfolioEmptyState() {
  return (
    <Card className="border-dashed bg-background">
      <CardHeader>
        <CardTitle>No portfolio cash data yet</CardTitle>
        <CardDescription>
          Add a balance to an account to start seeing your cash portfolio
          overview.
        </CardDescription>
      </CardHeader>
      <CardFooter className="justify-end">
        <Link className={cn(buttonVariants())} to="/accounts">
          View accounts
        </Link>
      </CardFooter>
    </Card>
  );
}

function formatDisplayAmount(amount: string, currency: string) {
  const value = Number(amount);

  if (Number.isNaN(value)) {
    return `${amount} ${currency}`;
  }

  return `${new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value)} ${currency}`;
}

function formatOriginalAmount(amount: string) {
  const value = Number(amount);
  if (Number.isNaN(value)) return amount;
  return new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 0,
    maximumFractionDigits: 8,
  }).format(value);
}

function formatTimestamp(timestamp: string) {
  return timestamp.slice(0, 16);
}
