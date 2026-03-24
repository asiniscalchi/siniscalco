import { useEffect, useState, type ReactNode } from "react";
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
import { getPortfolioApiUrl, type PortfolioSummaryResponse } from "@/lib/api";
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

type PortfolioSummary = PortfolioSummaryResponse;

const portfolioTitle = "Portfolio";

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
          throw new Error(
            `portfolio request failed with status ${response.status}`,
          );
        }

        const summary = (await response.json()) as PortfolioSummaryResponse;

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
        <PortfolioLoadingState />
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

function PortfolioPageHeader() {
  return (
    <header className="px-1">
      <h1 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
        {portfolioTitle}
      </h1>
    </header>
  );
}

function PortfolioLoadingState() {
  return (
    <div className="flex flex-col gap-4">
      <PortfolioPageHeader />
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
    </div>
  );
}

function PortfolioErrorState({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex flex-col gap-4">
      <PortfolioPageHeader />
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
    </div>
  );
}

function PortfolioReadyState({ summary }: { summary: PortfolioSummary }) {
  const { hideValues } = useUiState();
  const hasCashData = summary.cash_by_currency.length > 0;
  const totalValue = summary.total_value_amount
    ? Number(summary.total_value_amount)
    : null;

  if (!hasCashData) {
    return (
      <div className="flex flex-col gap-4">
        <PortfolioPageHeader />
        <PortfolioEmptyState />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-8">
      <PortfolioPageHeader />
      <PortfolioSummarySection summary={summary} hideValues={hideValues} />
      <div className="grid gap-8 lg:grid-cols-[1fr_350px]">
        <PortfolioAccountBreakdown
          accountTotals={summary.account_totals}
          hideValues={hideValues}
          totalValue={totalValue}
        />
        <PortfolioCurrencyBreakdown
          balances={summary.cash_by_currency}
          hideValues={hideValues}
          totalValue={totalValue}
        />
      </div>
    </div>
  );
}

function PortfolioSummarySection({
  summary,
  hideValues,
}: {
  summary: PortfolioSummary;
  hideValues: boolean;
}) {
  return (
    <section className="space-y-4">
      <div className="flex flex-col gap-1.5 px-1">
        <div className="flex flex-col items-baseline gap-1 sm:flex-row sm:gap-4">
          {summary.total_value_status === "ok" && summary.total_value_amount ? (
            <MoneyText
              className="text-3xl font-bold tracking-tight sm:text-4xl"
              currency={summary.display_currency}
              hidden={hideValues}
              value={summary.total_value_amount}
            />
          ) : (
            <span className="text-xl font-semibold text-muted-foreground sm:text-2xl">
              Conversion unavailable
            </span>
          )}
          <span className="text-sm font-medium text-muted-foreground">
            Total Cash Value
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          {summary.total_value_status === "conversion_unavailable" && (
            <span className="font-medium text-destructive">
              Conversion data unavailable
            </span>
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
              <span className="font-medium text-destructive">
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
  );
}

function PortfolioAccountBreakdown({
  accountTotals,
  hideValues,
  totalValue,
}: {
  accountTotals: PortfolioSummary["account_totals"];
  hideValues: boolean;
  totalValue: number | null;
}) {
  const sortedAccountTotals = [...accountTotals].sort((a, b) => {
    const valueA = a.total_amount ? Number(a.total_amount) : 0;
    const valueB = b.total_amount ? Number(b.total_amount) : 0;
    return valueB - valueA;
  });

  return (
    <Card className="self-start bg-background">
      <CardHeader className="border-b">
        <CardTitle>Cash By Account</CardTitle>
      </CardHeader>
      <CardContent className="pt-6">
        <div className="space-y-6">
          {sortedAccountTotals.map((account) => {
            const accountValue = account.total_amount ? Number(account.total_amount) : 0;
            const percentage =
              totalValue && account.summary_status === "ok"
                ? (accountValue / totalValue) * 100
                : 0;

            return (
              <PortfolioProgressItem
                key={account.id}
                label={account.name}
                meta={account.account_type}
                percentage={percentage}
                value={
                  account.summary_status === "ok" && account.total_amount ? (
                    <MoneyText
                      className="text-right text-xs text-muted-foreground"
                      currency={account.total_currency}
                      hidden={hideValues}
                      value={account.total_amount}
                    />
                  ) : (
                    <span className="text-xs font-medium text-muted-foreground">
                      Conversion unavailable
                    </span>
                  )
                }
              />
            );
          })}
        </div>
      </CardContent>
      {accountTotals.some((account) => account.summary_status !== "ok") ? (
        <CardFooter className="pt-0">
          <p className="text-xs italic text-muted-foreground">
            * Some accounts are hidden or incomplete due to missing conversion
            rates.
          </p>
        </CardFooter>
      ) : null}
    </Card>
  );
}

function PortfolioCurrencyBreakdown({
  balances,
  hideValues,
  totalValue,
}: {
  balances: PortfolioSummary["cash_by_currency"];
  hideValues: boolean;
  totalValue: number | null;
}) {
  return (
    <Card className="self-start bg-background">
      <CardHeader className="border-b">
        <CardTitle>Cash By Currency</CardTitle>
      </CardHeader>
      <CardContent className="pb-6 pt-6">
        <div className="space-y-6">
          {balances.map((balance) => {
            const balanceValue = balance.converted_amount
              ? Number(balance.converted_amount)
              : null;
            const percentage =
              totalValue && balanceValue ? (balanceValue / totalValue) * 100 : 0;

            return (
              <PortfolioProgressItem
                key={balance.currency}
                label={balance.currency}
                percentage={percentage}
                value={
                  <MoneyText
                    className="text-right text-xs text-muted-foreground"
                    currency={balance.currency}
                    hidden={hideValues}
                    maximumFractionDigits={8}
                    minimumFractionDigits={0}
                    value={balance.amount}
                  />
                }
              />
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}

function PortfolioProgressItem({
  label,
  meta,
  percentage,
  value,
}: {
  label: string;
  meta?: string;
  percentage: number;
  value: ReactNode;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-end justify-between text-sm">
        <div className="flex flex-col">
          <span className="font-medium">{label}</span>
          {meta ? <span className="text-xs text-muted-foreground opacity-80">{meta}</span> : null}
        </div>
        <div className="text-right">{value}</div>
      </div>
      <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
        <div
          className="h-full bg-primary transition-all duration-500"
          style={{ width: `${percentage}%` }}
        />
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

function formatTimestamp(timestamp: string) {
  const [date, time] = timestamp.split(" ");
  if (!date || !time) {
    return timestamp.slice(0, 16);
  }

  return `${date} ${time.slice(0, 5)}`;
}
