import { useState } from "react";
import type { FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useMutation, useQuery } from "@apollo/client/react";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { buttonVariants } from "@/components/ui/button-variants";
import {
  CREATE_ACCOUNT_MUTATION,
  CURRENCIES_QUERY,
  extractGqlErrorMessage,
  type AccountType,
} from "@/lib/api";
import { cn } from "@/lib/utils";

export function AccountNewPage() {
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [accountType, setAccountType] = useState<AccountType>("BANK");
  const [baseCurrency, setBaseCurrency] = useState("EUR");

  const { data: currenciesData, loading: currenciesLoading, error: currenciesError } = useQuery<{ currencies: string[] }>(CURRENCIES_QUERY);

  const [createAccount, { loading: submitting, error: submitError }] = useMutation(CREATE_ACCOUNT_MUTATION, {
    onCompleted: () => navigate("/accounts"),
  });

  const currencies = currenciesData?.currencies ?? [];

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void createAccount({
      variables: { input: { name: name.trim(), accountType, baseCurrency } },
    });
  }

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-6">
      <header className="flex flex-col gap-3 rounded-2xl border bg-background p-6 shadow-sm">
        <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
          Cash Accounts
        </p>
        <h1 className="text-3xl font-semibold tracking-tight">New Account</h1>
        <p className="max-w-xl text-sm text-muted-foreground">
          Create a cash account with its name, account type, and base currency.
        </p>
      </header>

      <Card className="bg-background">
        <CardContent>
          <form className="space-y-5" onSubmit={handleSubmit}>
            <div className="space-y-2">
              <label className="text-sm font-medium" htmlFor="account-name">
                Name
              </label>
              <input
                className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                id="account-name"
                name="name"
                onChange={(event) => setName(event.target.value)}
                placeholder="IBKR"
                required
                type="text"
                value={name}
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium" htmlFor="account-type">
                Account type
              </label>
              <select
                className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                id="account-type"
                name="account_type"
                onChange={(event) =>
                  setAccountType(event.target.value as AccountType)
                }
                value={accountType}
              >
                <option value="BANK">bank</option>
                <option value="BROKER">broker</option>
                <option value="CRYPTO">crypto</option>
              </select>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium" htmlFor="base-currency">
                Base currency
              </label>
              <select
                className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                id="base-currency"
                name="base_currency"
                onChange={(event) => setBaseCurrency(event.target.value)}
                required
                value={baseCurrency}
              >
                {currencies.map((code) => (
                  <option key={code} value={code}>
                    {code}
                  </option>
                ))}
              </select>
            </div>

            {currenciesError ? (
              <p className="text-sm text-destructive">
                {extractGqlErrorMessage(currenciesError, "Could not load currencies.")}
              </p>
            ) : null}

            {submitError ? (
              <p className="text-sm text-destructive">
                {extractGqlErrorMessage(submitError, "Could not create account.")}
              </p>
            ) : null}

            <div className="flex justify-end gap-3">
              <Link
                className={cn(buttonVariants({ variant: "outline" }))}
                to="/accounts"
              >
                Cancel
              </Link>
              <Button
                disabled={submitting || currenciesLoading || currencies.length === 0}
                type="submit"
              >
                {submitting ? "Creating..." : "Create account"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
