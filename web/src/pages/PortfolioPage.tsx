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
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
      <header className="rounded-xl border bg-background px-6 py-5 shadow-sm">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">Portfolio</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            Review total cash exposure across accounts and currencies.
          </p>
        </div>
      </header>

      {requestState.status === "loading" ? <PortfolioLoadingState /> : null}
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
          <CardContent>
            <div className="h-8 w-40 rounded-full bg-muted" />
          </CardContent>
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
    return <PortfolioEmptyState />;
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[1.3fr_0.9fr]">
      <div className="grid gap-4">
        <TotalValueCard summary={summary} />
        <AccountBreakdownCard summary={summary} />
      </div>
      <div className="grid gap-4">
        <CashByCurrencyCard summary={summary} />
        <FxStatusCard summary={summary} />
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

function TotalValueCard({ summary }: { summary: PortfolioSummary }) {
  return (
    <Card className="bg-background">
      <CardHeader>
        <CardTitle>Total Cash Value</CardTitle>
        <CardDescription>Converted to {summary.display_currency}</CardDescription>
      </CardHeader>
      <CardContent>
        {summary.total_value_status === "ok" && summary.total_value_amount ? (
          <>
            <p className="text-sm text-muted-foreground">Portfolio total</p>
            <p className="mt-2 text-3xl font-semibold tracking-tight">
              {formatDisplayAmount(
                summary.total_value_amount,
                summary.display_currency,
              )}
            </p>
          </>
        ) : (
          <>
            <p className="text-sm text-muted-foreground">Portfolio total</p>
            <p className="mt-2 text-lg font-semibold">Conversion unavailable</p>
            <p className="mt-2 text-sm text-muted-foreground">
              Some balances cannot be converted into {summary.display_currency}.
            </p>
          </>
        )}
      </CardContent>
    </Card>
  );
}

function AccountBreakdownCard({ summary }: { summary: PortfolioSummary }) {
  return (
    <Card className="bg-background">
      <CardHeader>
        <CardTitle>By Account</CardTitle>
        <CardDescription>Aggregated account totals in EUR</CardDescription>
      </CardHeader>
      <CardContent>
        <ul className="space-y-3" aria-label="Portfolio breakdown by account">
          {summary.account_totals.map((account) => (
            <li
              key={account.id}
              className="flex items-center justify-between gap-4 border-b pb-3 last:border-b-0 last:pb-0"
            >
              <div>
                <p className="font-medium">{account.name}</p>
                <p className="text-sm text-muted-foreground">
                  {account.account_type}
                </p>
              </div>
              {account.summary_status === "ok" && account.total_amount ? (
                <p className="font-semibold">
                  {formatDisplayAmount(account.total_amount, account.total_currency)}
                </p>
              ) : (
                <p className="text-sm font-medium text-muted-foreground">
                  Conversion unavailable
                </p>
              )}
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}

function CashByCurrencyCard({ summary }: { summary: PortfolioSummary }) {
  return (
    <Card className="bg-background">
      <CardHeader>
        <CardTitle>Cash By Currency</CardTitle>
        <CardDescription>Original balances across all accounts</CardDescription>
      </CardHeader>
      <CardContent>
        <ul className="space-y-3" aria-label="Cash grouped by currency">
          {summary.cash_by_currency.map((balance) => (
            <li
              key={balance.currency}
              className="flex items-center justify-between gap-4"
            >
              <span className="font-medium">{balance.currency}</span>
              <span className="font-mono text-sm">
                {formatOriginalAmount(balance.amount)} {balance.currency}
              </span>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}

function FxStatusCard({ summary }: { summary: PortfolioSummary }) {
  return (
    <Card className="bg-background">
      <CardHeader>
        <CardTitle>FX Status</CardTitle>
        <CardDescription>Portfolio-wide EUR conversion status</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {summary.total_value_status === "conversion_unavailable" ? (
          <p className="text-sm font-medium text-destructive">
            Conversion data unavailable
          </p>
        ) : null}
        <p className="text-sm text-muted-foreground">
          Last FX update:{" "}
          {summary.fx_last_updated ? formatTimestamp(summary.fx_last_updated) : "-"}
        </p>
      </CardContent>
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
  return amount.replace(/\.?0+$/, "").replace(/\.$/, "");
}

function formatTimestamp(timestamp: string) {
  return timestamp.slice(0, 16);
}
