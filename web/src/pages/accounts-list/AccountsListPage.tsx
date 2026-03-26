import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { BankIcon, BrokerIcon, CryptoIcon, PlusIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
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
              fxRates={requestState.fxRates}
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
                cashTotalAmount={portfolioAccount?.cash_total_amount ?? null}
                assetTotalAmount={portfolioAccount?.asset_total_amount ?? null}
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
  summaryStatus: "ok" | "conversion_unavailable";
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
          <div className="flex size-8 shrink-0 items-center justify-center rounded-lg border bg-muted/50 text-muted-foreground">
            {accountType === "bank" ? (
              <BankIcon />
            ) : accountType === "broker" ? (
              <BrokerIcon />
            ) : (
              <CryptoIcon />
            )}
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-baseline justify-between gap-4">
              <p className="truncate font-semibold">{name}</p>
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
            <div className="mt-0.5 flex items-center justify-between gap-4 text-xs text-muted-foreground">
              <span className="capitalize">
                {accountType} · {baseCurrency}
              </span>
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

function formatFxRate(rate: string) {
  const parsedRate = Number(rate);

  if (Number.isNaN(parsedRate)) {
    return rate;
  }

  return parsedRate.toFixed(4);
}
