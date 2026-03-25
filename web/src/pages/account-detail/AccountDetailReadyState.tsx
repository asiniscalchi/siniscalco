import { useEffect, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { buttonVariants } from "@/components/ui/button-variants";
import {
  getAccountBalanceApiUrl,
  getAccountDetailApiUrl,
  readApiErrorMessage,
} from "@/lib/api";
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

import type { AccountAsset, AccountDetail } from "./types";

type AccountDetailReadyStateProps = {
  account: AccountDetail;
  assets: AccountAsset[];
  currencies: string[];
  onDeleteSuccess: () => void;
  onRefresh: () => void;
};

export function AccountDetailReadyState({
  account,
  assets,
  currencies,
  onDeleteSuccess,
  onRefresh,
}: AccountDetailReadyStateProps) {
  const { hideValues } = useUiState();
  const [currency, setCurrency] = useState(account.base_currency);
  const [amount, setAmount] = useState("");
  const [requestState, setRequestState] = useState<
    | { status: "idle" }
    | { status: "submitting" }
    | { status: "error"; message: string }
  >({ status: "idle" });
  const [deletingCurrency, setDeletingCurrency] = useState<string | null>(null);
  const [isDeletingAccount, setIsDeletingAccount] = useState(false);

  useEffect(() => {
    setCurrency(account.base_currency);
    setAmount("");
    setRequestState({ status: "idle" });
    setDeletingCurrency(null);
    setIsDeletingAccount(false);
  }, [account.base_currency, account.id]);

  async function handleBalanceSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    setRequestState({ status: "submitting" });

    try {
      const response = await fetch(
        getAccountBalanceApiUrl(String(account.id), currency),
        {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            amount: amount.trim(),
          }),
        },
      );

      if (!response.ok) {
        const message = await readApiErrorMessage(
          response,
          "Could not save balance.",
        );
        throw new Error(message);
      }

      setAmount("");
      setRequestState({ status: "idle" });
      onRefresh();
    } catch (error) {
      setRequestState({
        status: "error",
        message:
          error instanceof Error ? error.message : "Could not save balance.",
      });
    }
  }

  async function handleDeleteBalance(balanceCurrency: string) {
    setDeletingCurrency(balanceCurrency);
    setRequestState({ status: "idle" });

    try {
      const response = await fetch(
        getAccountBalanceApiUrl(String(account.id), balanceCurrency),
        {
          method: "DELETE",
        },
      );

      if (!response.ok) {
        const message = await readApiErrorMessage(
          response,
          "Could not delete balance.",
        );
        throw new Error(message);
      }

      onRefresh();
    } catch (error) {
      setRequestState({
        status: "error",
        message:
          error instanceof Error ? error.message : "Could not delete balance.",
      });
    } finally {
      setDeletingCurrency(null);
    }
  }

  async function handleDeleteAccount() {
    setIsDeletingAccount(true);
    setRequestState({ status: "idle" });

    try {
      const response = await fetch(getAccountDetailApiUrl(String(account.id)), {
        method: "DELETE",
      });

      if (!response.ok) {
        const message = await readApiErrorMessage(
          response,
          "Could not delete account.",
        );
        throw new Error(message);
      }

      onDeleteSuccess();
    } catch (error) {
      setRequestState({
        status: "error",
        message:
          error instanceof Error ? error.message : "Could not delete account.",
      });
      setIsDeletingAccount(false);
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
      <header className="flex flex-col gap-4 rounded-2xl border bg-background p-6 shadow-sm sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-2">
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
            Cash Accounts
          </p>
          <h1 className="text-3xl font-semibold tracking-tight">
            {account.name}
          </h1>
          <p className="text-sm text-muted-foreground">
            {account.account_type} · base currency {account.base_currency}
          </p>
        </div>
        <Link
          className={cn(buttonVariants({ variant: "outline" }))}
          to="/accounts"
        >
          Back to accounts
        </Link>
      </header>

      <div className="flex justify-end">
        <Button
          disabled={isDeletingAccount}
          onClick={() => void handleDeleteAccount()}
          type="button"
          variant="destructive"
        >
          {isDeletingAccount ? "Deleting account..." : "Delete account"}
        </Button>
      </div>

      <Card className="bg-background">
        <CardHeader>
          <CardTitle>Account Summary</CardTitle>
          <CardDescription>Created at {account.created_at}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3 text-sm sm:grid-cols-3">
          <div className="rounded-xl border p-4">
            <p className="text-muted-foreground">Cash</p>
            {account.summary_status === "ok" && account.cash_total_amount && account.total_currency ? (
              <MoneyText
                className="mt-2 block font-semibold"
                currency={account.total_currency}
                hidden={hideValues}
                value={account.cash_total_amount}
              />
            ) : (
              <p className="mt-2 font-medium text-muted-foreground">Unavailable</p>
            )}
          </div>
          <div className="rounded-xl border p-4">
            <p className="text-muted-foreground">Assets</p>
            {account.summary_status === "ok" && account.asset_total_amount && account.total_currency ? (
              <MoneyText
                className="mt-2 block font-semibold"
                currency={account.total_currency}
                hidden={hideValues}
                value={account.asset_total_amount}
              />
            ) : (
              <p className="mt-2 font-medium text-muted-foreground">Unavailable</p>
            )}
          </div>
          <div className="rounded-xl border p-4">
            <p className="text-muted-foreground">Total</p>
            {account.summary_status === "ok" && account.total_amount && account.total_currency ? (
              <MoneyText
                className="mt-2 block font-semibold"
                currency={account.total_currency}
                hidden={hideValues}
                value={account.total_amount}
              />
            ) : (
              <p className="mt-2 font-medium text-muted-foreground">Unavailable</p>
            )}
          </div>
        </CardContent>
      </Card>

      <section className="space-y-4">
        <div className="space-y-1">
          <h2 className="text-xl font-semibold tracking-tight">Assets</h2>
          <p className="text-sm text-muted-foreground">
            Current asset positions held in this account.
          </p>
        </div>

        {assets.length === 0 ? (
          <Card className="border-dashed bg-background">
            <CardHeader>
              <CardTitle>No assets yet</CardTitle>
              <CardDescription>
                This account does not have any open asset positions yet.
              </CardDescription>
            </CardHeader>
          </Card>
        ) : (
          <Card className="bg-background">
            <CardContent className="pt-6">
              <div className="w-full overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                      <th className="pb-3 pr-4">Symbol</th>
                      <th className="pb-3 pr-4">Name</th>
                      <th className="pb-3 pr-4">Type</th>
                      <th className="pb-3 pr-4 text-right">Quantity</th>
                      <th className="pb-3 text-right">Value</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y">
                    {assets.map((asset) => (
                      <tr key={asset.asset_id}>
                        <td className="py-3 pr-4 font-bold tabular-nums">
                          {asset.symbol}
                        </td>
                        <td className="py-3 pr-4">{asset.name}</td>
                        <td className="py-3 pr-4">
                          <span className="inline-flex items-center rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide">
                            {asset.asset_type.replace("_", " ")}
                          </span>
                        </td>
                        <td className="py-3 pr-4 text-right font-mono tabular-nums">
                          {parseFloat(asset.quantity)}
                        </td>
                        <td className="py-3 text-right">
                          {asset.value ? (
                            <MoneyText
                              currency={account.base_currency}
                              hidden={hideValues}
                              value={asset.value}
                            />
                          ) : (
                            <span className="text-muted-foreground">—</span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </CardContent>
          </Card>
        )}
      </section>

      <section className="space-y-4">
        <div className="space-y-1">
          <h2 className="text-xl font-semibold tracking-tight">Balances</h2>
          <p className="text-sm text-muted-foreground">
            Current cash balance state for this account.
          </p>
        </div>

        <Card className="bg-background">
          <CardHeader>
            <CardTitle>Update Balance</CardTitle>
            <CardDescription>
              Create or update the current balance for one currency.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form className="space-y-4" onSubmit={handleBalanceSubmit}>
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <label
                    className="text-sm font-medium"
                    htmlFor="balance-currency"
                  >
                    Currency
                  </label>
                  <select
                    className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                    id="balance-currency"
                    onChange={(event) => setCurrency(event.target.value)}
                    required
                    value={currency}
                  >
                    {currencies.map((code) => (
                      <option key={code} value={code}>
                        {code}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="space-y-2">
                  <label
                    className="text-sm font-medium"
                    htmlFor="balance-amount"
                  >
                    Amount
                  </label>
                  <input
                    className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                    id="balance-amount"
                    onChange={(event) => setAmount(event.target.value)}
                    placeholder="12000.00000000"
                    required
                    type="text"
                    value={amount}
                  />
                </div>
              </div>

              {requestState.status === "error" ? (
                <p className="text-sm text-destructive">
                  {requestState.message}
                </p>
              ) : null}

              <div className="flex justify-end">
                <Button
                  disabled={requestState.status === "submitting"}
                  type="submit"
                >
                  {requestState.status === "submitting"
                    ? "Saving..."
                    : "Save balance"}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>

        {account.balances.length === 0 ? (
          <Card className="border-dashed bg-background">
            <CardHeader>
              <CardTitle>No balances yet</CardTitle>
              <CardDescription>
                This account does not have any stored balances yet.
              </CardDescription>
            </CardHeader>
          </Card>
        ) : (
          <div className="grid gap-3">
            {account.balances.map((balance) => (
              <Card className="bg-background" key={balance.currency}>
                <CardHeader>
                  <CardTitle>{balance.currency}</CardTitle>
                  <CardDescription>
                    Updated at {balance.updated_at}
                  </CardDescription>
                  <CardAction>
                    <Button
                      disabled={deletingCurrency === balance.currency}
                      onClick={() => void handleDeleteBalance(balance.currency)}
                      type="button"
                      variant="outline"
                    >
                      {deletingCurrency === balance.currency
                        ? "Deleting..."
                        : "Delete"}
                    </Button>
                  </CardAction>
                </CardHeader>
                <CardContent>
                  <MoneyText
                    className="text-2xl font-semibold tracking-tight"
                    hidden={hideValues}
                    includeCurrency={false}
                    maximumFractionDigits={8}
                    minimumFractionDigits={8}
                    value={balance.amount}
                  />
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
