import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { buttonVariants } from "@/components/ui/button-variants";
import {
  getAccountsApiUrl,
  getFxRatesApiUrl,
  type FxRateSummaryResponse,
} from "@/lib/api";
import { cn } from "@/lib/utils";

type AccountSummary = {
  id: number;
  name: string;
  account_type: string;
  base_currency: string;
  summary_status: "ok" | "conversion_unavailable";
  total_amount: string | null;
  total_currency: string | null;
};

type FxRateSummary = {
  target_currency: string;
  rates: {
    currency: string;
    rate: string;
  }[];
  last_updated: string | null;
  refresh_status: "available" | "unavailable";
  refresh_error: string | null;
};

export function AccountsListPage() {
  const [requestState, setRequestState] = useState<
    | { status: "loading" }
    | { status: "error" }
    | { status: "ready"; accounts: AccountSummary[]; fxRates: FxRateSummary }
  >({ status: "loading" });
  const [retryToken, setRetryToken] = useState(0);

  useEffect(() => {
    let cancelled = false;

    async function loadAccounts() {
      setRequestState({ status: "loading" });

      try {
        const [accountsResponse, fxRatesResponse] = await Promise.all([
          fetch(getAccountsApiUrl()),
          fetch(getFxRatesApiUrl()),
        ]);

        if (!accountsResponse.ok) {
          throw new Error(
            `accounts request failed with status ${accountsResponse.status}`,
          );
        }

        if (!fxRatesResponse.ok) {
          throw new Error(
            `fx rates request failed with status ${fxRatesResponse.status}`,
          );
        }

        const [accounts, fxRates] = await Promise.all([
          accountsResponse.json() as Promise<AccountSummary[]>,
          fxRatesResponse.json() as Promise<FxRateSummaryResponse>,
        ]);

        if (cancelled) {
          return;
        }

        setRequestState({ status: "ready", accounts, fxRates });
      } catch {
        if (!cancelled) {
          setRequestState({ status: "error" });
        }
      }
    }

    void loadAccounts();

    return () => {
      cancelled = true;
    };
  }, [retryToken]);

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
      <header className="flex flex-col gap-4 rounded-xl border bg-background px-6 py-5 shadow-sm sm:flex-row sm:items-center sm:justify-between">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">Accounts</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            View your cash accounts and manage their details.
          </p>
        </div>
        <Link className={cn(buttonVariants({ size: "lg" }))} to="/accounts/new">
          Create account
        </Link>
      </header>

      <section className="space-y-4">
        {requestState.status === "loading" ? <AccountsLoadingState /> : null}
        {requestState.status === "error" ? (
          <AccountsErrorState
            onRetry={() => setRetryToken((value) => value + 1)}
          />
        ) : null}
        {requestState.status === "ready" ? (
          <AccountsReadyState
            accounts={requestState.accounts}
            fxRates={requestState.fxRates}
          />
        ) : null}
      </section>
    </div>
  );
}

function AccountsLoadingState() {
  return (
    <div className="grid gap-3">
      {Array.from({ length: 3 }).map((_, index) => (
        <Card key={index} className="border-dashed bg-background/70">
          <CardHeader>
            <div className="h-5 w-32 rounded-full bg-muted" />
            <div className="h-4 w-24 rounded-full bg-muted" />
          </CardHeader>
          <CardContent>
            <div className="h-4 w-20 rounded-full bg-muted" />
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function AccountsEmptyState() {
  return (
    <Card className="border-dashed bg-background">
      <CardHeader>
        <CardTitle>No accounts yet</CardTitle>
        <CardDescription>
          Create your first cash account to start managing account details.
        </CardDescription>
      </CardHeader>
      <CardFooter className="justify-end">
        <Link className={cn(buttonVariants())} to="/accounts/new">
          Create account
        </Link>
      </CardFooter>
    </Card>
  );
}

function AccountsErrorState({ onRetry }: { onRetry: () => void }) {
  return (
    <Card className="border-destructive/30 bg-background">
      <CardHeader>
        <CardTitle>Could not load accounts</CardTitle>
        <CardDescription>
          The accounts list request failed. Try again to reload the page data.
        </CardDescription>
      </CardHeader>
      <CardFooter className="justify-end gap-3">
        <Link
          className={cn(buttonVariants({ variant: "outline" }))}
          to="/accounts/new"
        >
          Create account
        </Link>
        <Button onClick={onRetry} type="button">
          Retry
        </Button>
      </CardFooter>
    </Card>
  );
}

function AccountsReadyState({
  accounts,
  fxRates,
}: {
  accounts: AccountSummary[];
  fxRates: FxRateSummary;
}) {
  return (
    <>
      {accounts.length === 0 ? (
        <AccountsEmptyState />
      ) : (
        <div className="grid gap-3">
          {accounts.map((account) => (
            <AccountListItem
              key={account.id}
              id={String(account.id)}
              name={account.name}
              accountType={account.account_type}
              baseCurrency={account.base_currency}
              summaryStatus={account.summary_status}
              totalAmount={account.total_amount}
              totalCurrency={account.total_currency}
            />
          ))}
        </div>
      )}
      <FxRatesFooter summary={fxRates} />
    </>
  );
}

function formatAmount(amount: number | string, currency: string) {
  const value = typeof amount === "string" ? Number(amount) : amount;

  if (Number.isNaN(value)) {
    return `${amount} ${currency}`;
  }

  const formatted = new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);

  return `${formatted} ${currency}`;
}

function FxRatesFooter({ summary }: { summary: FxRateSummary }) {
  if (summary.rates.length === 0) {
    return null;
  }

  return (
    <footer
      className="mt-8 flex flex-wrap items-center justify-between border-t py-4 text-[11px] font-mono text-muted-foreground/60"
      aria-label={`FX rates against ${summary.target_currency}`}
    >
      {summary.rates.map((rate) => (
        <div key={rate.currency} className="flex items-center gap-1.5">
          <span className="font-bold">{rate.currency}</span>
          <span>{formatFxRate(rate.rate)}</span>
        </div>
      ))}
      {summary.refresh_status === "unavailable" && (
        <div
          className="text-destructive/80 font-bold uppercase tracking-wider"
          title={summary.refresh_error || "FX refresh unavailable"}
        >
          Refresh Failed
        </div>
      )}
    </footer>
  );
}

function AccountListItem({
  id,
  name,
  accountType,
  baseCurrency,
  summaryStatus,
  totalAmount,
  totalCurrency,
}: {
  id: string;
  name: string;
  accountType: string;
  baseCurrency: string;
  summaryStatus: "ok" | "conversion_unavailable";
  totalAmount: string | null;
  totalCurrency: string | null;
}) {
  return (
    <Link className="block" to={`/accounts/${id}`}>
      <Card className="bg-background transition-colors hover:bg-muted/30">
        <CardHeader>
          <CardTitle>{name}</CardTitle>
          <CardDescription className="flex items-center gap-2">
            {accountType}
            <span className="text-muted-foreground/50">·</span>
            {baseCurrency}
          </CardDescription>
          <CardAction>
            <div
              className={cn(
                buttonVariants({ variant: "ghost", size: "sm" }),
                "text-muted-foreground",
              )}
            >
              View details
            </div>
          </CardAction>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">Total</p>
          {summaryStatus === "ok" && totalAmount && totalCurrency ? (
            <p className="mt-1 text-lg font-semibold">
              {formatAmount(totalAmount, totalCurrency)}
            </p>
          ) : (
            <p className="mt-1 text-sm font-medium text-muted-foreground">
              Conversion unavailable
            </p>
          )}
        </CardContent>
      </Card>
    </Link>
  );
}

function formatFxRate(rate: string) {
  const parsedRate = Number(rate);

  if (Number.isNaN(parsedRate)) {
    return rate;
  }

  return parsedRate.toFixed(4);
}
