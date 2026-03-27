import { useEffect, useState, type FormEvent } from "react";
import { createPortal } from "react-dom";
import { gql } from "@apollo/client/core";
import { useMutation } from "@apollo/client/react";

import { Button } from "@/components/ui/button";
import { extractGqlErrorMessage } from "@/lib/gql";

const CREATE_TRANSACTION_MUTATION = gql`
  mutation CreateTransaction($input: CreateTransactionInput!) {
    createTransaction(input: $input) {
      id accountId assetId transactionType tradeDate
      quantity unitPrice currencyCode notes
    }
  }
`;

const UPDATE_TRANSACTION_MUTATION = gql`
  mutation UpdateTransaction($id: Int!, $input: UpdateTransactionInput!) {
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

function getTodayDate() {
  return new Date().toISOString().split("T")[0];
}

function getDefaultCurrency(accounts: Account[], selectedAccountId: string) {
  if (!selectedAccountId) {
    return "";
  }

  return (
    accounts.find((account) => String(account.id) === selectedAccountId)
      ?.baseCurrency ?? ""
  );
}

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
    currency: getDefaultCurrency(accounts, selectedAccountId),
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

  const [createTransaction, { loading: creating }] = useMutation(CREATE_TRANSACTION_MUTATION);
  const [updateTransaction, { loading: updating }] = useMutation(UPDATE_TRANSACTION_MUTATION);
  const isSubmitting = creating || updating;

  useEffect(() => {
    if (!open) {
      return;
    }

    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, [open]);

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
      onSaved();
    } catch (error) {
      setSubmitError(extractGqlErrorMessage(
        error,
        transactionId ? "Failed to update transaction" : "Failed to create transaction",
      ));
    }
  };

  return createPortal(
    <div
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center overflow-y-auto bg-black/40 p-4 backdrop-blur-sm animate-in fade-in duration-200"
      role="dialog"
    >
      <div className="my-auto flex max-h-full w-full max-w-2xl flex-col overflow-hidden rounded-xl border bg-background shadow-2xl animate-in zoom-in-95 duration-200">
        <header className="flex-none border-b px-6 py-4">
          <h2 className="text-lg font-semibold">
            {transactionId ? "Edit Transaction" : "Add Transaction"}
          </h2>
          <p className="text-sm text-muted-foreground">
            {transactionId
              ? "Update transaction details."
              : `Record a new transaction for ${selectedAccount?.name || "selected account"}.`}
          </p>
        </header>
        <form className="flex flex-1 flex-col overflow-hidden" onSubmit={handleSubmit}>
          <div className="grid flex-1 min-h-0 gap-5 overflow-y-auto px-6 py-6 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <label
                className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                htmlFor="asset-select"
              >
                Asset *
              </label>
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
            </div>
            <div className="flex flex-col gap-1.5">
              <label
                className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                htmlFor="type-select"
              >
                Type *
              </label>
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
            </div>
            <div className="flex flex-col gap-1.5">
              <label
                className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                htmlFor="trade-date-input"
              >
                Trade Date *
              </label>
              <input
                required
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="trade-date-input"
                onChange={(event) => updateField("tradeDate", event.target.value)}
                type="date"
                value={formState.tradeDate}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <label
                className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                htmlFor="quantity-input"
              >
                Quantity *
              </label>
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
            </div>
            <div className="flex flex-col gap-1.5">
              <label
                className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                htmlFor="price-input"
              >
                Unit Price *
              </label>
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
            </div>
            <div className="flex flex-col gap-1.5">
              <label
                className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                htmlFor="currency-input"
              >
                Currency *
              </label>
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
            </div>
            <div className="flex flex-col gap-1.5 sm:col-span-2">
              <label
                className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                htmlFor="notes-input"
              >
                Notes
              </label>
              <input
                className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                id="notes-input"
                onChange={(event) => updateField("notes", event.target.value)}
                placeholder="Optional notes"
                type="text"
                value={formState.notes}
              />
            </div>
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
      </div>
    </div>,
    document.body,
  );
}
