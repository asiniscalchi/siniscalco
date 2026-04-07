import { useState, type FormEvent } from "react";
import { gql } from "@apollo/client/core";
import { useMutation, useApolloClient } from "@apollo/client/react";

import { FormField } from "@/components/FormField";
import { ModalDialog } from "@/components/ModalDialog";
import { Button } from "@/components/ui/button";
import { extractGqlErrorMessage } from "@/lib/gql";
import { getAccountCurrency, getTodayDate } from "@/lib/form-utils";
import { useBodyScrollLock } from "@/lib/use-body-scroll-lock";

const CREATE_TRANSACTION_MUTATION = gql`
  mutation CreateTransaction($input: TransactionInput!) {
    createTransaction(input: $input) {
      id accountId assetId transactionType tradeDate
      quantity unitPrice currencyCode notes
    }
  }
`;

const UPDATE_TRANSACTION_MUTATION = gql`
  mutation UpdateTransaction($id: Int!, $input: TransactionInput!) {
    updateTransaction(id: $id, input: $input) {
      id accountId assetId transactionType tradeDate
      quantity unitPrice currencyCode notes
    }
  }
`;

import type { Account, Asset, Transaction } from "./types";

type TransactionFormModalProps = {
  open: boolean;
  accounts: Account[];
  assets: Asset[];
  selectedAccountId: string;
  editingTransaction: Transaction | null;
  onClose: () => void;
  onSaved: () => void;
};

type FormState = {
  assetId: string;
  type: "BUY" | "SELL";
  tradeDate: string;
  quantity: string;
  unitPrice: string;
  currency: string;
  notes: string;
};

function buildCreateState(
  accounts: Account[],
  selectedAccountId: string,
): FormState {
  return {
    assetId: "",
    type: "BUY",
    tradeDate: getTodayDate(),
    quantity: "",
    unitPrice: "",
    currency: getAccountCurrency(accounts, selectedAccountId),
    notes: "",
  };
}

function buildEditState(transaction: Transaction): FormState {
  return {
    assetId: String(transaction.assetId),
    type: transaction.transactionType,
    tradeDate: transaction.tradeDate,
    quantity: transaction.quantity,
    unitPrice: transaction.unitPrice,
    currency: transaction.currencyCode,
    notes: transaction.notes || "",
  };
}

export function TransactionFormModal({
  open,
  accounts,
  assets,
  selectedAccountId,
  editingTransaction,
  onClose,
  onSaved,
}: TransactionFormModalProps) {
  const [formState, setFormState] = useState<FormState>(() =>
    editingTransaction
      ? buildEditState(editingTransaction)
      : buildCreateState(accounts, selectedAccountId),
  );
  const [submitError, setSubmitError] = useState<string | null>(null);

  const client = useApolloClient();
  const [createTransaction, { loading: creating }] = useMutation(CREATE_TRANSACTION_MUTATION);
  const [updateTransaction, { loading: updating }] = useMutation(UPDATE_TRANSACTION_MUTATION);
  const isSubmitting = creating || updating;

  useBodyScrollLock(open);

  if (!open) {
    return null;
  }

  const selectedAccount = selectedAccountId
    ? accounts.find((account) => String(account.id) === selectedAccountId)
    : null;
  const transactionId = editingTransaction?.id ?? null;
  const transactionSubmitDisabled =
    isSubmitting || assets.length === 0 || !formState.assetId;

  const updateField = <K extends keyof FormState>(field: K, value: FormState[K]) => {
    setFormState((current) => ({ ...current, [field]: value }));
  };

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setSubmitError(null);

    try {
      if (transactionId) {
        await updateTransaction({
          variables: {
            id: transactionId,
            input: {
              accountId: editingTransaction!.accountId,
              assetId: parseInt(formState.assetId),
              transactionType: formState.type,
              tradeDate: formState.tradeDate,
              quantity: formState.quantity,
              unitPrice: formState.unitPrice,
              currencyCode: formState.currency,
              notes: formState.notes || null,
            },
          },
        });
      } else {
        await createTransaction({
          variables: {
            input: {
              accountId: parseInt(selectedAccountId),
              assetId: parseInt(formState.assetId),
              transactionType: formState.type,
              tradeDate: formState.tradeDate,
              quantity: formState.quantity,
              unitPrice: formState.unitPrice,
              currencyCode: formState.currency,
              notes: formState.notes || null,
            },
          },
        });
      }
      client.cache.evict({ fieldName: "assets" });
      client.cache.evict({ fieldName: "portfolio" });
      client.cache.gc();
      onSaved();
    } catch (error) {
      setSubmitError(extractGqlErrorMessage(
        error,
        transactionId ? "Failed to update transaction" : "Failed to create transaction",
      ));
    }
  };

  return (
    <ModalDialog
      description={
        transactionId
          ? "Update transaction details."
          : `Record a new transaction for ${selectedAccount?.name || "selected account"}.`
      }
      title={transactionId ? "Edit Transaction" : "Add Transaction"}
    >
      <form className="flex flex-1 flex-col overflow-hidden" onSubmit={handleSubmit}>
          <div className="grid flex-1 min-h-0 gap-5 overflow-y-auto px-6 py-6 sm:grid-cols-2">
            <FormField htmlFor="asset-select" label="Asset *">
              <select
                required
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                disabled={assets.length === 0}
                id="asset-select"
                onChange={(event) => updateField("assetId", event.target.value)}
                value={formState.assetId}
              >
                <option value="">Select asset...</option>
                {assets.map((asset) => (
                  <option key={asset.id} value={String(asset.id)}>
                    {asset.symbol} — {asset.name}
                  </option>
                ))}
              </select>
            </FormField>
            <FormField htmlFor="type-select" label="Type *">
              <select
                required
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="type-select"
                onChange={(event) =>
                  updateField("type", event.target.value as "BUY" | "SELL")
                }
                value={formState.type}
              >
                <option value="BUY">BUY</option>
                <option value="SELL">SELL</option>
              </select>
            </FormField>
            <FormField htmlFor="trade-date-input" label="Trade Date *">
              <input
                required
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="trade-date-input"
                onChange={(event) => updateField("tradeDate", event.target.value)}
                type="date"
                value={formState.tradeDate}
              />
            </FormField>
            <FormField htmlFor="quantity-input" label="Quantity *">
              <input
                required
                className="rounded-md border bg-background px-3 py-2 text-sm font-mono shadow-sm"
                id="quantity-input"
                min="0.00000001"
                onChange={(event) => updateField("quantity", event.target.value)}
                placeholder="0.00"
                step="any"
                type="number"
                value={formState.quantity}
              />
            </FormField>
            <FormField htmlFor="price-input" label="Unit Price *">
              <input
                required
                className="rounded-md border bg-background px-3 py-2 text-sm font-mono shadow-sm"
                id="price-input"
                min="0"
                onChange={(event) => updateField("unitPrice", event.target.value)}
                placeholder="0.00"
                step="any"
                type="number"
                value={formState.unitPrice}
              />
            </FormField>
            <FormField htmlFor="currency-input" label="Currency *">
              <input
                required
                className="rounded-md border bg-background px-3 py-2 text-sm font-mono uppercase shadow-sm"
                id="currency-input"
                maxLength={3}
                onChange={(event) => updateField("currency", event.target.value)}
                placeholder="USD"
                type="text"
                value={formState.currency}
              />
            </FormField>
            <FormField className="sm:col-span-2" htmlFor="notes-input" label="Notes">
              <input
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="notes-input"
                onChange={(event) => updateField("notes", event.target.value)}
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
            <Button disabled={transactionSubmitDisabled} type="submit">
              {isSubmitting
                ? transactionId
                  ? "Saving..."
                  : "Adding..."
                : transactionId
                  ? "Save Changes"
                  : "Add Transaction"}
            </Button>
          </footer>
        </form>
    </ModalDialog>
  );
}
