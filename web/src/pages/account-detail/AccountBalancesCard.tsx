import { useState, type FormEvent } from "react";
import { gql } from "@apollo/client/core";
import { useQuery, useMutation } from "@apollo/client/react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { extractGqlErrorMessage } from "@/lib/gql";
import { type AccountBalancesQuery } from "@/gql/types";

const ACCOUNT_QUERY = gql`
  query AccountBalances($id: Int!) {
    account(id: $id) {
      id baseCurrency
      balances { currency amount updatedAt }
    }
  }
`;

const CURRENCIES_QUERY = gql`
  query BalanceCurrencies {
    currencies
  }
`;

const UPSERT_BALANCE_MUTATION = gql`
  mutation UpsertBalance($accountId: Int!, $input: UpsertBalanceInput!) {
    upsertBalance(accountId: $accountId, input: $input) {
      currency amount updatedAt
    }
  }
`;

const DELETE_BALANCE_MUTATION = gql`
  mutation DeleteBalance($accountId: Int!, $currency: String!) {
    deleteBalance(accountId: $accountId, currency: $currency)
  }
`;
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";

import { type BalanceCurrenciesQuery } from "@/gql/types";

type AccountBalancesCardProps = {
  accountId: number;
  baseCurrency: string;
};

export function AccountBalancesCard({ accountId, baseCurrency }: AccountBalancesCardProps) {
  const { hideValues } = useUiState();
  const [currency, setCurrency] = useState(baseCurrency);
  const [amount, setAmount] = useState("");
  const [balanceError, setBalanceError] = useState<string | null>(null);
  const [deletingCurrency, setDeletingCurrency] = useState<string | null>(null);

  const { data: accountData, refetch } = useQuery<AccountBalancesQuery>(
    ACCOUNT_QUERY,
    { variables: { id: accountId } },
  );

  const { data: currenciesData } = useQuery<BalanceCurrenciesQuery>(CURRENCIES_QUERY);

  const [upsertBalance, { loading: savingBalance }] = useMutation(UPSERT_BALANCE_MUTATION);
  const [deleteBalance] = useMutation(DELETE_BALANCE_MUTATION);

  const account = accountData?.account;
  const currencies = currenciesData?.currencies ?? [];
  const balances = account?.balances ?? [];

  async function handleBalanceSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBalanceError(null);

    try {
      await upsertBalance({
        variables: { accountId, input: { currency, amount: amount.trim() } },
      });
      setAmount("");
      void refetch();
    } catch (error) {
      setBalanceError(extractGqlErrorMessage(error, "Could not save balance."));
    }
  }

  async function handleDeleteBalance(balanceCurrency: string) {
    setDeletingCurrency(balanceCurrency);
    setBalanceError(null);

    try {
      await deleteBalance({ variables: { accountId, currency: balanceCurrency } });
      void refetch();
    } catch (error) {
      setBalanceError(extractGqlErrorMessage(error, "Could not delete balance."));
    } finally {
      setDeletingCurrency(null);
    }
  }

  return (
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

            {balanceError ? (
              <p className="text-sm text-destructive">{balanceError}</p>
            ) : null}

            <div className="flex justify-end">
              <Button disabled={savingBalance} type="submit">
                {savingBalance ? "Saving..." : "Save balance"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      {balances.length === 0 ? (
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
          {balances.map((balance) => (
            <Card className="bg-background" key={balance.currency}>
              <CardHeader>
                <CardTitle>{balance.currency}</CardTitle>
                <CardDescription>
                  Updated at {balance.updatedAt}
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
  );
}
