import { useState, type FormEvent } from "react";
import { gql } from "@apollo/client/core";
import { useMutation } from "@apollo/client/react";

import { ModalDialog } from "@/components/ModalDialog";
import { Button } from "@/components/ui/button";
import { extractGqlErrorMessage, extractGqlFieldErrors } from "@/lib/gql";
import { useBodyScrollLock } from "@/lib/use-body-scroll-lock";
import { type AssetType, type AssetsQuery } from "@/gql/types";

const CREATE_ASSET_MUTATION = gql`
  mutation CreateAsset($input: AssetInput!) {
    createAsset(input: $input) {
      id symbol name assetType quoteSymbol isin
      currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
    }
  }
`;

const UPDATE_ASSET_MUTATION = gql`
  mutation UpdateAsset($id: Int!, $input: AssetInput!) {
    updateAsset(id: $id, input: $input) {
      id symbol name assetType quoteSymbol isin
      currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
    }
  }
`;

type AssetFormModalProps = {
  open: boolean;
  editingAsset: AssetsQuery["assets"][number] | null;
  onClose: () => void;
  onSaved: () => void;
};

type FormState = {
  symbol: string;
  name: string;
  type: AssetType;
  quoteSymbol: string;
  isin: string;
};

const initialFormState: FormState = {
  symbol: "",
  name: "",
  type: "STOCK",
  quoteSymbol: "",
  isin: "",
};

export function AssetFormModal({
  open,
  editingAsset,
  onClose,
  onSaved,
}: AssetFormModalProps) {
  const [formState, setFormState] = useState<FormState>(
    editingAsset
      ? {
          symbol: editingAsset.symbol,
          name: editingAsset.name,
          type: editingAsset.assetType,
          quoteSymbol: editingAsset.quoteSymbol || "",
          isin: editingAsset.isin || "",
        }
      : initialFormState,
  );
  const [fieldErrors, setFieldErrors] = useState<Record<string, string[]>>({});
  const [submitError, setSubmitError] = useState<string | null>(null);

  const [createAsset, { loading: creating }] = useMutation(CREATE_ASSET_MUTATION);
  const [updateAsset, { loading: updating }] = useMutation(UPDATE_ASSET_MUTATION);
  const isSubmitting = creating || updating;

  useBodyScrollLock(open);

  if (!open) {
    return null;
  }

  const getFieldError = (field: string) => fieldErrors[field]?.[0] ?? null;

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setFieldErrors({});
    setSubmitError(null);

    try {
      if (editingAsset) {
        await updateAsset({
          variables: {
            id: editingAsset.id,
            input: {
              symbol: formState.symbol,
              name: formState.name,
              assetType: formState.type,
              quoteSymbol: formState.quoteSymbol || null,
              isin: formState.isin || null,
            },
          },
        });
      } else {
        await createAsset({
          variables: {
            input: {
              symbol: formState.symbol,
              name: formState.name,
              assetType: formState.type,
              quoteSymbol: formState.quoteSymbol || null,
              isin: formState.isin || null,
            },
          },
        });
      }
      onSaved();
    } catch (error) {
      const fieldErrs = extractGqlFieldErrors(error);
      if (fieldErrs) {
        setFieldErrors(fieldErrs);
      }
      setSubmitError(
        extractGqlErrorMessage(error, editingAsset ? "Failed to update asset" : "Failed to create asset"),
      );
    }
  };

  return (
    <ModalDialog
      description={editingAsset ? "Update asset details." : "Add a new asset to use in transactions."}
      size="md"
      title={editingAsset ? "Edit Asset" : "Add Asset"}
    >
      <form className="flex flex-1 flex-col overflow-hidden" onSubmit={handleSubmit}>
          <div className="grid gap-4 overflow-y-auto px-6 py-6">
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <div className="flex flex-col gap-1.5">
                <label
                  className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                  htmlFor="asset-symbol"
                >
                  Symbol *
                </label>
                <input
                  autoFocus
                  required
                  className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                  id="asset-symbol"
                  onChange={(event) =>
                    setFormState((current) => ({
                      ...current,
                      symbol: event.target.value,
                    }))
                  }
                  placeholder="AAPL"
                  value={formState.symbol}
                />
                {getFieldError("symbol") ? (
                  <p className="text-xs text-destructive">{getFieldError("symbol")}</p>
                ) : null}
              </div>
              <div className="flex flex-col gap-1.5">
                <label
                  className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                  htmlFor="asset-name"
                >
                  Name *
                </label>
                <input
                  required
                  className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                  id="asset-name"
                  onChange={(event) =>
                    setFormState((current) => ({
                      ...current,
                      name: event.target.value,
                    }))
                  }
                  placeholder="Apple Inc."
                  value={formState.name}
                />
                {getFieldError("name") ? (
                  <p className="text-xs text-destructive">{getFieldError("name")}</p>
                ) : null}
              </div>
              <div className="flex flex-col gap-1.5">
                <label
                  className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                  htmlFor="asset-type"
                >
                  Asset Type *
                </label>
                <select
                  required
                  className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                  id="asset-type"
                  onChange={(event) =>
                    setFormState((current) => ({
                      ...current,
                      type: event.target.value as AssetType,
                    }))
                  }
                  value={formState.type}
                >
                  <option value="STOCK">STOCK</option>
                  <option value="ETF">ETF</option>
                  <option value="BOND">BOND</option>
                  <option value="CRYPTO">CRYPTO</option>
                  <option value="CASH_EQUIVALENT">CASH_EQUIVALENT</option>
                  <option value="OTHER">OTHER</option>
                </select>
                {getFieldError("asset_type") ? (
                  <p className="text-xs text-destructive">
                    {getFieldError("asset_type")}
                  </p>
                ) : null}
              </div>
              <div className="flex flex-col gap-1.5">
                <label
                  className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                  htmlFor="asset-quote-symbol"
                >
                  Quote Symbol
                </label>
                <input
                  className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                  id="asset-quote-symbol"
                  onChange={(event) =>
                    setFormState((current) => ({
                      ...current,
                      quoteSymbol: event.target.value,
                    }))
                  }
                  placeholder="AAPL or BTC/USD"
                  value={formState.quoteSymbol}
                />
                <p className="text-xs text-muted-foreground">
                  Optional override for the market data provider symbol.
                </p>
                {getFieldError("quote_symbol") ? (
                  <p className="text-xs text-destructive">
                    {getFieldError("quote_symbol")}
                  </p>
                ) : null}
              </div>
              <div className="flex flex-col gap-1.5">
                <label
                  className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                  htmlFor="asset-isin"
                >
                  ISIN
                </label>
                <input
                  className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                  id="asset-isin"
                  onChange={(event) =>
                    setFormState((current) => ({
                      ...current,
                      isin: event.target.value,
                    }))
                  }
                  placeholder="US0378331005"
                  value={formState.isin}
                />
              </div>
            </div>
            {submitError ? (
              <div className="rounded-md border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                {submitError}
              </div>
            ) : null}
          </div>
          <footer className="flex flex-none justify-end gap-3 rounded-b-xl border-t bg-muted/30 px-6 py-4">
            <Button onClick={onClose} type="button" variant="outline">
              Cancel
            </Button>
            <Button disabled={isSubmitting} type="submit">
              {isSubmitting
                ? editingAsset
                  ? "Saving..."
                  : "Adding..."
                : editingAsset
                  ? "Save Changes"
                  : "Add Asset"}
            </Button>
          </footer>
        </form>
    </ModalDialog>
  );
}
