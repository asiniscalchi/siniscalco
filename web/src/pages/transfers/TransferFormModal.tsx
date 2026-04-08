import { useState, type FormEvent } from "react";
import { gql } from "@apollo/client/core";
import { useMutation } from "@apollo/client/react";

import { FormField } from "@/components/FormField";
import { ModalDialog } from "@/components/ModalDialog";
import { Button } from "@/components/ui/button";
import { extractGqlErrorMessage } from "@/lib/gql";
import { getAccountCurrency, getTodayDate } from "@/lib/form-utils";
import { useBodyScrollLock } from "@/lib/use-body-scroll-lock";

const CREATE_TRANSFER_MUTATION = gql`
  mutation CreateTransfer($input: TransferInput!) {
    createTransfer(input: $input) {
      id fromAccountId toAccountId
      fromCurrency fromAmount toCurrency toAmount
      transferDate notes
    }
  }
`;

type TransferFormModalProps = {
  open: boolean;
  accounts: Account[];
  initialFromAccountId?: string;
  onClose: () => void;
  onSaved: () => void;
};

type FormState = {
  fromAccountId: string;
  toAccountId: string;
  fromCurrency: string;
  fromAmount: string;
  toCurrency: string;
  toAmount: string;
  transferDate: string;
  notes: string;
};

type Account = {
  id: number;
  name: string;
  baseCurrency: string;
};


export function TransferFormModal({
  open,
  accounts,
  initialFromAccountId = "",
  onClose,
  onSaved,
}: TransferFormModalProps) {
  const [formState, setFormState] = useState<FormState>({
    fromAccountId: initialFromAccountId,
    toAccountId: "",
    fromCurrency: getAccountCurrency(accounts, initialFromAccountId),
    fromAmount: "",
    toCurrency: "",
    toAmount: "",
    transferDate: getTodayDate(),
    notes: "",
  });
  const [submitError, setSubmitError] = useState<string | null>(null);

  const [createTransfer, { loading: creating }] = useMutation(CREATE_TRANSFER_MUTATION);

  useBodyScrollLock(open);

  if (!open) return null;

  const updateField = <K extends keyof FormState>(field: K, value: FormState[K]) => {
    setFormState((current) => ({ ...current, [field]: value }));
  };

  const handleFromAccountChange = (accountId: string) => {
    const currency = getAccountCurrency(accounts, accountId);
    setFormState((current) => ({
      ...current,
      fromAccountId: accountId,
      fromCurrency: currency,
    }));
  };

  const handleToAccountChange = (accountId: string) => {
    const currency = getAccountCurrency(accounts, accountId);
    setFormState((current) => ({
      ...current,
      toAccountId: accountId,
      toCurrency: currency,
    }));
  };

  const isSameCurrency = formState.fromCurrency === formState.toCurrency && formState.fromCurrency !== "";
  const submitDisabled =
    creating ||
    !formState.fromAccountId ||
    !formState.toAccountId ||
    !formState.fromAmount ||
    !formState.toAmount ||
    !formState.fromCurrency ||
    !formState.toCurrency;

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setSubmitError(null);

    try {
      await createTransfer({
        variables: {
          input: {
            fromAccountId: parseInt(formState.fromAccountId),
            toAccountId: parseInt(formState.toAccountId),
            fromCurrency: formState.fromCurrency.toUpperCase(),
            fromAmount: formState.fromAmount,
            toCurrency: formState.toCurrency.toUpperCase(),
            toAmount: formState.toAmount,
            transferDate: formState.transferDate,
            notes: formState.notes || null,
          },
        },
      });
      onSaved();
    } catch (error) {
      setSubmitError(extractGqlErrorMessage(error, "Failed to create transfer"));
    }
  };

  return (
    <ModalDialog description="Move funds between accounts." title="New Transfer">
      <form className="flex flex-1 flex-col overflow-hidden" onSubmit={handleSubmit}>
          <div className="grid flex-1 min-h-0 gap-5 overflow-y-auto px-6 py-6 sm:grid-cols-2">
            <FormField htmlFor="from-account-select" label="From Account *">
              <select
                required
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="from-account-select"
                onChange={(e) => handleFromAccountChange(e.target.value)}
                value={formState.fromAccountId}
              >
                <option value="">Select account...</option>
                {accounts.map((account) => (
                  <option key={account.id} value={String(account.id)}>
                    {account.name} ({account.baseCurrency})
                  </option>
                ))}
              </select>
            </FormField>
            <FormField htmlFor="to-account-select" label="To Account *">
              <select
                required
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="to-account-select"
                onChange={(e) => handleToAccountChange(e.target.value)}
                value={formState.toAccountId}
              >
                <option value="">Select account...</option>
                {accounts
                  .filter((a) => String(a.id) !== formState.fromAccountId)
                  .map((account) => (
                    <option key={account.id} value={String(account.id)}>
                      {account.name} ({account.baseCurrency})
                    </option>
                  ))}
              </select>
            </FormField>
            <FormField htmlFor="from-amount-input" label="Amount Sent *">
              <div className="flex gap-2">
                <input
                  required
                  className="min-w-0 flex-1 rounded-md border bg-background px-3 py-2 text-sm font-mono shadow-sm"
                  id="from-amount-input"
                  min="0.000001"
                  onChange={(e) => {
                    updateField("fromAmount", e.target.value);
                    if (isSameCurrency) {
                      updateField("toAmount", e.target.value);
                    }
                  }}
                  placeholder="0.00"
                  step="any"
                  type="number"
                  value={formState.fromAmount}
                />
                <input
                  required
                  className="w-20 rounded-md border bg-background px-3 py-2 text-sm font-mono uppercase shadow-sm"
                  maxLength={3}
                  onChange={(e) => updateField("fromCurrency", e.target.value)}
                  placeholder="EUR"
                  type="text"
                  value={formState.fromCurrency}
                />
              </div>
            </FormField>
            <FormField htmlFor="to-amount-input" label="Amount Received *">
              <div className="flex gap-2">
                <input
                  required
                  className="min-w-0 flex-1 rounded-md border bg-background px-3 py-2 text-sm font-mono shadow-sm"
                  id="to-amount-input"
                  min="0.000001"
                  onChange={(e) => updateField("toAmount", e.target.value)}
                  placeholder="0.00"
                  step="any"
                  type="number"
                  value={formState.toAmount}
                />
                <input
                  required
                  className="w-20 rounded-md border bg-background px-3 py-2 text-sm font-mono uppercase shadow-sm"
                  maxLength={3}
                  onChange={(e) => updateField("toCurrency", e.target.value)}
                  placeholder="USD"
                  type="text"
                  value={formState.toCurrency}
                />
              </div>
            </FormField>
            <FormField htmlFor="transfer-date-input" label="Date *">
              <input
                required
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="transfer-date-input"
                onChange={(e) => updateField("transferDate", e.target.value)}
                type="date"
                value={formState.transferDate}
              />
            </FormField>
            <FormField htmlFor="notes-input" label="Notes">
              <input
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="notes-input"
                onChange={(e) => updateField("notes", e.target.value)}
                placeholder="Optional notes"
                type="text"
                value={formState.notes}
              />
            </FormField>
            {submitError && (
              <div className="col-span-full rounded-md border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                {submitError}
              </div>
            )}
          </div>
          <footer className="flex flex-none justify-end gap-3 rounded-b-xl border-t bg-muted/30 px-6 py-4">
            <Button onClick={onClose} type="button" variant="outline">
              Cancel
            </Button>
            <Button disabled={submitDisabled} type="submit">
              {creating ? "Transferring..." : "Transfer"}
            </Button>
          </footer>
        </form>
    </ModalDialog>
  );
}
