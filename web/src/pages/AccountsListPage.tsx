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
  getPortfolioApiUrl,
  type FxRateSummaryResponse,
  type PortfolioSummaryResponse,
} from "@/lib/api";
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";
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
  const { hideValues } = useUiState();
  const [requestState, setRequestState] = useState<
    | { status: "loading" }
    | { status: "error" }
    | {
        status: "ready";
        accounts: AccountSummary[];
        fxRates: FxRateSummary;
        portfolio: PortfolioSummaryResponse;
      }
  >({ status: "loading" });
  const [retryToken, setRetryToken] = useState(0);

  useEffect(() => {
    let cancelled = false;

    async function loadData() {
      setRequestState({ status: "loading" });

      try {
        const [accountsResponse, fxRatesResponse, portfolioResponse] =
          await Promise.all([
            fetch(getAccountsApiUrl()),
            fetch(getFxRatesApiUrl()),
            fetch(getPortfolioApiUrl()),
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

        if (!portfolioResponse.ok) {
          throw new Error(
            `portfolio request failed with status ${portfolioResponse.status}`,
          );
        }

        const [accounts, fxRates, portfolio] = await Promise.all([
          accountsResponse.json() as Promise<AccountSummary[]>,
          fxRatesResponse.json() as Promise<FxRateSummaryResponse>,
          portfolioResponse.json() as Promise<PortfolioSummaryResponse>,
        ]);

        if (cancelled) {
          return;
        }

        setRequestState({ status: "ready", accounts, fxRates, portfolio });
      } catch {
        if (!cancelled) {
          setRequestState({ status: "error" });
        }
      }
    }

    void loadData();

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

      {requestState.status === "ready" && (
        <section className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Card className="bg-background">
            <CardHeader className="pb-2">
              <CardDescription className="text-xs font-medium uppercase tracking-wider">
                Total Accounts
              </CardDescription>
              <CardTitle className="text-2xl font-bold">
                {requestState.accounts.length}
              </CardTitle>
            </CardHeader>
          </Card>
          <Card className="bg-background">
            <CardHeader className="pb-2">
              <CardDescription className="text-xs font-medium uppercase tracking-wider">
                Combined Balance
              </CardDescription>
              <CardTitle className="text-2xl font-bold">
                {requestState.portfolio.total_value_status === "ok" &&
                requestState.portfolio.total_value_amount ? (
                  <MoneyText
                    currency={requestState.portfolio.display_currency}
                    hidden={hideValues}
                    value={requestState.portfolio.total_value_amount}
                  />
                ) : (
                  <span className="text-lg text-muted-foreground">
                    Unavailable
                  </span>
                )}
              </CardTitle>
            </CardHeader>
          </Card>
        </section>
      )}

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
            portfolio={requestState.portfolio}
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
  portfolio,
}: {
  accounts: AccountSummary[];
  fxRates: FxRateSummary;
  portfolio: PortfolioSummaryResponse;
}) {
  const { hideValues } = useUiState();

  return (
    <>
      {accounts.length === 0 ? (
        <AccountsEmptyState />
      ) : (
        <div className="grid gap-3">
          {accounts.map((account) => {
            const portfolioAccount = portfolio.account_totals.find(
              (a) => a.id === account.id,
            );
            const totalValue = portfolio.total_value_amount
              ? Number(portfolio.total_value_amount)
              : 0;
            const accountValue = portfolioAccount?.total_amount
              ? Number(portfolioAccount.total_amount)
              : 0;
            const percentage =
              totalValue > 0 && portfolioAccount?.summary_status === "ok"
                ? (accountValue / totalValue) * 100
                : 0;

            return (
              <AccountListItem
                key={account.id}
                id={String(account.id)}
                name={account.name}
                accountType={account.account_type}
                baseCurrency={account.base_currency}
                summaryStatus={account.summary_status}
                totalAmount={account.total_amount}
                totalCurrency={account.total_currency}
                hideValues={hideValues}
                weight={percentage}
              />
            );
          })}
        </div>
      )}
      <FxRatesFooter summary={fxRates} />
    </>
  );
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
  hideValues,
  weight,
}: {
  id: string;
  name: string;
  accountType: string;
  baseCurrency: string;
  summaryStatus: "ok" | "conversion_unavailable";
  totalAmount: string | null;
  totalCurrency: string | null;
  hideValues: boolean;
  weight: number;
}) {
  return (
    <Link className="block" to={`/accounts/${id}`}>
      <Card className="bg-background transition-colors hover:bg-muted/30">
        <CardHeader className="flex-row items-start gap-4 space-y-0">
          <div className="mt-1 flex size-10 shrink-0 items-center justify-center rounded-xl border bg-muted/50 text-muted-foreground">
            {accountType === "bank" ? <BankIcon /> : <BrokerIcon />}
          </div>
          <div className="flex-1 space-y-1">
            <CardTitle className="text-xl">{name}</CardTitle>
            <CardDescription className="flex items-center gap-2">
              <span className="capitalize">{accountType}</span>
              <span className="text-muted-foreground/50">·</span>
              {baseCurrency}
            </CardDescription>
          </div>
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
        <CardContent className="space-y-4">
          <div className="flex items-end justify-between">
            <div>
              <p className="text-sm text-muted-foreground">Total balance</p>
              {summaryStatus === "ok" && totalAmount && totalCurrency ? (
                <p className="mt-0.5 text-2xl font-bold tracking-tight">
                  <MoneyText
                    className="text-left"
                    currency={totalCurrency}
                    hidden={hideValues}
                    value={totalAmount}
                  />
                </p>
              ) : (
                <p className="mt-0.5 text-sm font-medium text-muted-foreground">
                  Conversion unavailable
                </p>
              )}
            </div>
            {summaryStatus === "ok" && (
              <div className="text-right">
                <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  Weight
                </p>
                <p className="mt-0.5 font-mono text-sm font-semibold">
                  {weight.toFixed(1)}%
                </p>
              </div>
            )}
          </div>
          {summaryStatus === "ok" && (
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
              <div
                className="h-full bg-primary transition-all duration-500"
                style={{ width: `${weight}%` }}
              />
            </div>
          )}
        </CardContent>
      </Card>
    </Link>
  );
}

function BankIcon() {
  return (
    <svg
      className="size-5"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M3 21h18" />
      <path d="M3 10h18" />
      <path d="M5 6l7-3 7 3" />
      <path d="M4 10v11" />
      <path d="M20 10v11" />
      <path d="M8 14v3" />
      <path d="M12 14v3" />
      <path d="M16 14v3" />
    </svg>
  );
}

function BrokerIcon() {
  return (
    <svg
      className="size-5"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M12 2v20" />
      <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
    </svg>
  );
}

function formatFxRate(rate: string) {
  const parsedRate = Number(rate);

  if (Number.isNaN(parsedRate)) {
    return rate;
  }

  return parsedRate.toFixed(4);
}

