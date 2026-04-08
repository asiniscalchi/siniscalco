import { useState, type FormEvent } from "react";
import { gql } from "@apollo/client/core";
import { useMutation, useQuery } from "@apollo/client/react";

import { FormField } from "@/components/FormField";
import { ModalDialog } from "@/components/ModalDialog";
import { Button } from "@/components/ui/button";
import type {
  BalanceCurrenciesQuery,
  CreateCashMovementMutation,
  CreateCashMovementMutationVariables,
} from "@/gql/types";
import { extractGqlErrorMessage } from "@/lib/gql";
import { getTodayDate } from "@/lib/form-utils";
import { useBodyScrollLock } from "@/lib/use-body-scroll-lock";

import type { Account } from "./types";

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

type CashMovementKind = "deposit" | "withdraw";

type CashMovementFormModalProps = {
  account: Account | null;
  kind: CashMovementKind;
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
};

type FormState = {
  currency: string;
  amount: string;
  date: string;
  notes: string;
};

function buildCreateState(account: Account | null): FormState {
  return {
    currency: account?.baseCurrency ?? "",
    amount: "",
    date: getTodayDate(),
    notes: "",
  };
}

export function CashMovementFormModal({
  account,
  kind,
  open,
  onClose,
  onSaved,
}: CashMovementFormModalProps) {
  const [formState, setFormState] = useState<FormState>(() => buildCreateState(account));
  const [submitError, setSubmitError] = useState<string | null>(null);

  const { data: currenciesData } = useQuery<BalanceCurrenciesQuery>(CURRENCIES_QUERY, {
    skip: !open || account === null,
  });
  const [createCashMovement, { loading: saving }] = useMutation<
    CreateCashMovementMutation,
    CreateCashMovementMutationVariables
  >(CREATE_CASH_MOVEMENT_MUTATION);

  useBodyScrollLock(open);

  if (!open || account === null) return null;

  const currencies = currenciesData?.currencies ?? [];
  const amountLabel = kind === "deposit" ? "Deposit Amount *" : "Withdrawal Amount *";
  const submitLabel = kind === "deposit" ? "Record Deposit" : "Record Withdrawal";
  const description =
    kind === "deposit"
      ? `Add cash to ${account.name}.`
      : `Remove cash from ${account.name}.`;

  const updateField = <K extends keyof FormState>(field: K, value: FormState[K]) => {
    setFormState((current) => ({ ...current, [field]: value }));
  };

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setSubmitError(null);

    try {
      const normalizedAmount = formState.amount.trim();
      const signedAmount =
        kind === "deposit" || normalizedAmount.startsWith("-")
          ? normalizedAmount
          : `-${normalizedAmount}`;

      await createCashMovement({
        variables: {
          accountId: account.id,
          input: {
            currency: formState.currency,
            amount: signedAmount,
            date: formState.date,
            notes: formState.notes || null,
          },
        },
      });
      onSaved();
    } catch (error) {
      setSubmitError(extractGqlErrorMessage(error, `Failed to record ${kind}`));
    }
  };

  return (
    <ModalDialog description={description} title={kind === "deposit" ? "Record Deposit" : "Record Withdrawal"}>
      <form className="flex flex-1 flex-col overflow-hidden" onSubmit={handleSubmit}>
        <div className="grid flex-1 min-h-0 gap-5 overflow-y-auto px-6 py-6 sm:grid-cols-2">
          <FormField htmlFor="cash-currency-select" label="Currency *">
            <select
              required
              className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
              id="cash-currency-select"
              onChange={(event) => updateField("currency", event.target.value)}
              value={formState.currency}
            >
              {currencies.length === 0 ? (
                <option value={formState.currency}>{formState.currency}</option>
              ) : null}
              {currencies.map((currency) => (
                <option key={currency} value={currency}>
                  {currency}
                </option>
              ))}
            </select>
          </FormField>
          <FormField htmlFor="cash-amount-input" label={amountLabel}>
            <input
              required
              className="rounded-md border bg-background px-3 py-2 text-sm font-mono shadow-sm"
              id="cash-amount-input"
              min="0.000001"
              onChange={(event) => updateField("amount", event.target.value)}
              placeholder="0.00"
              step="any"
              type="number"
              value={formState.amount}
            />
          </FormField>
          <FormField htmlFor="cash-date-input" label="Date *">
            <input
              required
              className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
              id="cash-date-input"
              onChange={(event) => updateField("date", event.target.value)}
              type="date"
              value={formState.date}
            />
          </FormField>
          <FormField htmlFor="cash-notes-input" label="Notes">
            <input
              className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
              id="cash-notes-input"
              onChange={(event) => updateField("notes", event.target.value)}
              placeholder="Optional notes"
              type="text"
              value={formState.notes}
            />
          </FormField>
          {submitError ? (
            <div className="col-span-full rounded-md border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {submitError}
            </div>
          ) : null}
        </div>
        <footer className="flex flex-none justify-end gap-3 rounded-b-xl border-t bg-muted/30 px-6 py-4">
          <Button onClick={onClose} type="button" variant="outline">
            Cancel
          </Button>
          <Button disabled={saving} type="submit">
            {saving ? "Saving..." : submitLabel}
          </Button>
        </footer>
      </form>
    </ModalDialog>
  );
}
