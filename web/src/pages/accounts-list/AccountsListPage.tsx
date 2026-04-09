import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";
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
import { type AccountsListQuery, type AccountsListPortfolioQuery } from "@/gql/types";

const ACCOUNTS_QUERY = gql`
  query AccountsList {
    accounts {
      id name accountType baseCurrency summaryStatus
      cashTotalAmount assetTotalAmount totalAmount totalCurrency
    }
  }
`;

const PORTFOLIO_QUERY = gql`
  query AccountsListPortfolio {
    portfolio {
      displayCurrency totalValueAmount
      accountTotals {
        id summaryStatus
        cashTotalAmount assetTotalAmount totalAmount
      }
    }
  }
`;
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

export function AccountsListPage() {
  const { data: accountsData, loading: accountsLoading, error: accountsError, refetch: refetchAccounts } = useQuery<AccountsListQuery>(ACCOUNTS_QUERY, { fetchPolicy: "cache-and-network" });
  const { data: portfolioData, loading: portfolioLoading, error: portfolioError, refetch: refetchPortfolio } = useQuery<AccountsListPortfolioQuery>(PORTFOLIO_QUERY, { fetchPolicy: "cache-and-network" });

  const loading = accountsLoading || portfolioLoading;
  const error = accountsError ?? portfolioError;

  function handleRetry() {
    void refetchAccounts();
    void refetchPortfolio();
  }

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
          {loading ? <AccountsLoadingState /> : null}
          {!loading && error ? (
            <AccountsErrorState onRetry={handleRetry} />
          ) : null}
          {!loading && !error && accountsData && portfolioData ? (
            <AccountsReadyState
              accounts={accountsData.accounts}
              portfolio={portfolioData.portfolio}
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
  accounts: AccountsListQuery["accounts"];
  portfolio: AccountsListPortfolioQuery["portfolio"];
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
              totalValue > 0 && portfolioAccount?.summaryStatus === "OK"
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
              {summaryStatus === "OK" && totalAmount && totalCurrency ? (
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
              {summaryStatus === "OK" && totalCurrency && (
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
            {summaryStatus === "OK" && (
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
