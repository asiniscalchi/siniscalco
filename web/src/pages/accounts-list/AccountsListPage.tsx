import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { ItemLabel } from "@/components/ItemLabel";
import { PlusIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
} from "@/components/ui/card";
import { buttonVariants } from "@/components/ui/button-variants";
import {
  fetchAccounts,
  fetchPortfolio,
  type AccountSummary,
  type PortfolioSummary,
} from "@/lib/api";
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

export function AccountsListPage() {
  const [requestState, setRequestState] = useState<
    | { status: "loading" }
    | { status: "error" }
    | {
        status: "ready";
        accounts: AccountSummary[];
        portfolio: PortfolioSummary;
      }
  >({ status: "loading" });
  const [retryToken, setRetryToken] = useState(0);

  useEffect(() => {
    let cancelled = false;

    async function loadData() {
      setRequestState({ status: "loading" });

      try {
        const [accounts, portfolio] = await Promise.all([
          fetchAccounts(),
          fetchPortfolio(),
        ]);

        if (cancelled) {
          return;
        }

        setRequestState({ status: "ready", accounts, portfolio });
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
      <Card className="bg-background">
        <CardHeader className="pb-2">
          <div className="flex items-center gap-2">
            <h1 className="flex-1 text-2xl font-semibold tracking-tight">Accounts</h1>
            <Link
              aria-label="Create account"
              className={cn(buttonVariants({ size: "icon-lg" }))}
              title="Create account"
              to="/accounts/new"
            >
              <PlusIcon />
            </Link>
          </div>
        </CardHeader>
        <CardContent className="pt-4">
          {requestState.status === "loading" ? <AccountsLoadingState /> : null}
          {requestState.status === "error" ? (
            <AccountsErrorState
              onRetry={() => setRetryToken((value) => value + 1)}
            />
          ) : null}
          {requestState.status === "ready" ? (
            <AccountsReadyState
              accounts={requestState.accounts}
              portfolio={requestState.portfolio}
            />
          ) : null}
        </CardContent>
      </Card>
    </div>
  );
}

function AccountsLoadingState() {
  return (
    <div className="grid gap-3">
      {Array.from({ length: 3 }).map((_, index) => (
        <div key={index} className="rounded-xl border border-dashed bg-background/70 p-4">
          <div className="h-5 w-32 rounded-full bg-muted" />
          <div className="mt-2 h-4 w-24 rounded-full bg-muted" />
          <div className="mt-3 h-4 w-20 rounded-full bg-muted" />
        </div>
      ))}
    </div>
  );
}

function AccountsEmptyState() {
  return (
    <div className="py-8 text-center">
      <p className="font-medium">No accounts yet</p>
      <p className="mt-1 text-sm text-muted-foreground">
        Create your first cash account to start managing account details.
      </p>
    </div>
  );
}

function AccountsErrorState({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="py-4">
      <p className="font-medium text-destructive">Could not load accounts</p>
      <p className="mt-1 text-sm text-muted-foreground">
        The accounts list request failed. Try again to reload the page data.
      </p>
      <div className="mt-4 flex justify-end">
        <Button onClick={onRetry} size="lg" type="button">
          Retry
        </Button>
      </div>
    </div>
  );
}

function AccountsReadyState({
  accounts,
  portfolio,
}: {
  accounts: AccountSummary[];
  portfolio: PortfolioSummary;
}) {
  const { hideValues } = useUiState();

  return (
    <>
      {accounts.length === 0 ? (
        <AccountsEmptyState />
      ) : (
        <div className="grid gap-3">
          {accounts.map((account) => {
            const portfolioAccount = portfolio.accountTotals.find(
              (a) => a.id === account.id,
            );
            const totalValue = portfolio.totalValueAmount
              ? Number(portfolio.totalValueAmount)
              : 0;
            const accountValue = portfolioAccount?.totalAmount
              ? Number(portfolioAccount.totalAmount)
              : 0;
            const percentage =
              totalValue > 0 && portfolioAccount?.summaryStatus === "ok"
                ? (accountValue / totalValue) * 100
                : 0;

            return (
              <AccountListItem
                key={account.id}
                id={String(account.id)}
                name={account.name}
                accountType={account.accountType}
                baseCurrency={account.baseCurrency}
                summaryStatus={account.summaryStatus}
                cashTotalAmount={portfolioAccount?.cashTotalAmount ?? null}
                assetTotalAmount={portfolioAccount?.assetTotalAmount ?? null}
                totalAmount={account.totalAmount}
                totalCurrency={account.totalCurrency}
                hideValues={hideValues}
                weight={percentage}
              />
            );
          })}
        </div>
      )}
    </>
  );
}

function AccountListItem({
  id,
  name,
  accountType,
  baseCurrency,
  summaryStatus,
  cashTotalAmount,
  assetTotalAmount,
  totalAmount,
  totalCurrency,
  hideValues,
  weight,
}: {
  id: string;
  name: string;
  accountType: string;
  baseCurrency: string;
  summaryStatus: string;
  cashTotalAmount: string | null;
  assetTotalAmount: string | null;
  totalAmount: string | null;
  totalCurrency: string | null;
  hideValues: boolean;
  weight: number;
}) {
  return (
    <Link className="block" to={`/accounts/${id}`}>
      <Card className="bg-background transition-colors hover:bg-muted/30">
        <CardContent className="flex items-center gap-3 py-4">
          <div className="min-w-0 flex-1">
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <ItemLabel
                  primary={name}
                  secondary={`${accountType} · ${baseCurrency}`}
                />
              </div>
              {summaryStatus === "ok" && totalAmount && totalCurrency ? (
                <MoneyText
                  className="shrink-0 font-semibold tabular-nums"
                  currency={totalCurrency}
                  hidden={hideValues}
                  value={totalAmount}
                />
              ) : (
                <p className="shrink-0 text-sm text-muted-foreground">
                  Unavailable
                </p>
              )}
            </div>
            <div className="mt-0.5 flex items-center justify-end gap-4 text-xs text-muted-foreground">
              {summaryStatus === "ok" && totalCurrency && (
                <span className="flex shrink-0 gap-3">
                  <span>
                    Cash{" "}
                    {cashTotalAmount ? (
                      <MoneyText
                        className="font-medium text-foreground"
                        currency={totalCurrency}
                        hidden={hideValues}
                        value={cashTotalAmount}
                      />
                    ) : (
                      "—"
                    )}
                  </span>
                  <span>
                    Assets{" "}
                    {assetTotalAmount ? (
                      <MoneyText
                        className="font-medium text-foreground"
                        currency={totalCurrency}
                        hidden={hideValues}
                        value={assetTotalAmount}
                      />
                    ) : (
                      "—"
                    )}
                  </span>
                </span>
              )}
            </div>
            {summaryStatus === "ok" && (
              <div className="mt-2 h-1 w-full overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full bg-primary transition-all duration-500"
                  style={{ width: `${weight}%` }}
                />
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </Link>
  );
}
