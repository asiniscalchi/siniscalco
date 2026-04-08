import { useState } from "react";
import { gql } from "@apollo/client/core";
import { useMutation } from "@apollo/client/react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { extractGqlErrorMessage } from "@/lib/gql";

const DELETE_ACCOUNT_MUTATION = gql`
  mutation DeleteAccount($id: Int!) {
    deleteAccount(id: $id)
  }
`;
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";

import { AccountAssetsCard } from "./AccountAssetsCard";
import { AccountBalancesCard } from "./AccountBalancesCard";
import type { AccountDetail } from "./types";

type AccountDetailReadyStateProps = {
  account: AccountDetail;
  onDeleteSuccess: () => void;
};

export function AccountDetailReadyState({
  account,
  onDeleteSuccess,
}: AccountDetailReadyStateProps) {
  const { hideValues } = useUiState();
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [deleteAccount, { loading: isDeletingAccount }] = useMutation(DELETE_ACCOUNT_MUTATION);

  async function handleDeleteAccount() {
    setDeleteError(null);
    try {
      await deleteAccount({ variables: { id: account.id } });
      onDeleteSuccess();
    } catch (error) {
      setDeleteError(extractGqlErrorMessage(error, "Could not delete account."));
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
      <header className="rounded-2xl border bg-background p-6 shadow-sm">
        <div className="space-y-2">
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
            Cash Accounts
          </p>
          <h1 className="text-3xl font-semibold tracking-tight">
            {account.name}
          </h1>
          <p className="text-sm text-muted-foreground">
            {account.accountType} · base currency {account.baseCurrency}
          </p>
        </div>
      </header>

      <div className="flex flex-col items-end gap-2">
        <Button
          disabled={isDeletingAccount}
          onClick={() => void handleDeleteAccount()}
          type="button"
          variant="destructive"
        >
          {isDeletingAccount ? "Deleting account..." : "Delete account"}
        </Button>
        {deleteError ? (
          <p className="text-sm text-destructive">{deleteError}</p>
        ) : null}
      </div>

      <Card className="bg-background">
        <CardHeader>
          <CardTitle>Account Summary</CardTitle>
          <CardDescription>Created at {account.createdAt}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3 text-sm sm:grid-cols-3">
          <div className="rounded-xl border p-4">
            <p className="text-muted-foreground">Cash</p>
            {account.summaryStatus === "OK" && account.cashTotalAmount && account.totalCurrency ? (
              <MoneyText
                className="mt-2 block font-semibold"
                currency={account.totalCurrency}
                hidden={hideValues}
                value={account.cashTotalAmount}
              />
            ) : (
              <p className="mt-2 font-medium text-muted-foreground">Unavailable</p>
            )}
          </div>
          <div className="rounded-xl border p-4">
            <p className="text-muted-foreground">Assets</p>
            {account.summaryStatus === "OK" && account.assetTotalAmount && account.totalCurrency ? (
              <MoneyText
                className="mt-2 block font-semibold"
                currency={account.totalCurrency}
                hidden={hideValues}
                value={account.assetTotalAmount}
              />
            ) : (
              <p className="mt-2 font-medium text-muted-foreground">Unavailable</p>
            )}
          </div>
          <div className="rounded-xl border p-4">
            <p className="text-muted-foreground">Total</p>
            {account.summaryStatus === "OK" && account.totalAmount && account.totalCurrency ? (
              <MoneyText
                className="mt-2 block font-semibold"
                currency={account.totalCurrency}
                hidden={hideValues}
                value={account.totalAmount}
              />
            ) : (
              <p className="mt-2 font-medium text-muted-foreground">Unavailable</p>
            )}
          </div>
        </CardContent>
      </Card>

      <AccountAssetsCard accountId={account.id} baseCurrency={account.baseCurrency} />

      <AccountBalancesCard accountId={account.id} />
    </div>
  );
}
