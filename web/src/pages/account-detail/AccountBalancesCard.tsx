import { useState, type FormEvent } from "react";
import { gql } from "@apollo/client/core";
import { useQuery, useMutation } from "@apollo/client/react";

import { Button } from "@/components/ui/button";
import {
  Card,
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

const CREATE_CASH_MOVEMENT_MUTATION = gql`
  mutation CreateCashMovement($accountId: Int!, $input: CashMovementInput!) {
    createCashMovement(accountId: $accountId, input: $input) {
      currency amount date
    }
  }
`;
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";

import { type BalanceCurrenciesQuery } from "@/gql/types";

type AccountBalancesCardProps = {
  accountId: number;
  baseCurrency: string;
};

function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

export function AccountBalancesCard({ accountId, baseCurrency }: AccountBalancesCardProps) {
  const { hideValues } = useUiState();
  const [currency, setCurrency] = useState(baseCurrency);
  const [amount, setAmount] = useState("");
  const [date, setDate] = useState(todayIso());
  const [movementError, setMovementError] = useState<string | null>(null);

  const { data: accountData, refetch } = useQuery<AccountBalancesQuery>(
    ACCOUNT_QUERY,
    { variables: { id: accountId } },
  );

  const { data: currenciesData } = useQuery<BalanceCurrenciesQuery>(CURRENCIES_QUERY);

  const [createCashMovement, { loading: saving }] = useMutation(CREATE_CASH_MOVEMENT_MUTATION);

  const account = accountData?.account;
  const currencies = currenciesData?.currencies ?? [];
  const balances = account?.balances ?? [];

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setMovementError(null);

    try {
      await createCashMovement({
        variables: { accountId, input: { currency, amount: amount.trim(), date } },
      });
      setAmount("");
      void refetch();
    } catch (error) {
      setMovementError(extractGqlErrorMessage(error, "Could not record cash movement."));
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
          <CardTitle>Record Cash Movement</CardTitle>
          <CardDescription>
            Record a deposit (positive) or withdrawal (negative) for one currency.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form className="space-y-4" onSubmit={handleSubmit}>
            <div className="grid gap-4 sm:grid-cols-3">
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
                  placeholder="1000.00 or -500.00"
                  required
                  type="text"
                  value={amount}
                />
              </div>
              <div className="space-y-2">
                <label
                  className="text-sm font-medium"
                  htmlFor="balance-date"
                >
                  Date
                </label>
                <input
                  className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                  id="balance-date"
                  onChange={(event) => setDate(event.target.value)}
                  required
                  type="date"
                  value={date}
                />
              </div>
            </div>

            {movementError ? (
              <p className="text-sm text-destructive">{movementError}</p>
            ) : null}

            <div className="flex justify-end">
              <Button disabled={saving} type="submit">
                {saving ? "Saving..." : "Record movement"}
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
              This account does not have any recorded cash movements yet.
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
                  Last movement at {balance.updatedAt}
                </CardDescription>
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
