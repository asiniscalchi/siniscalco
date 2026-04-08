import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { type AccountBalancesQuery } from "@/gql/types";

const ACCOUNT_QUERY = gql`
  query AccountBalances($id: Int!) {
    account(id: $id) {
      id
      balances { currency amount updatedAt }
    }
  }
`;

import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";

type AccountBalancesCardProps = {
  accountId: number;
};

export function AccountBalancesCard({ accountId }: AccountBalancesCardProps) {
  const { hideValues } = useUiState();

  const { data: accountData } = useQuery<AccountBalancesQuery>(
    ACCOUNT_QUERY,
    { variables: { id: accountId } },
  );

  const account = accountData?.account;
  const balances = account?.balances ?? [];

  return (
    <section className="space-y-4">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold tracking-tight">Balances</h2>
        <p className="text-sm text-muted-foreground">
          Current cash balance state for this account.
        </p>
      </div>

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
