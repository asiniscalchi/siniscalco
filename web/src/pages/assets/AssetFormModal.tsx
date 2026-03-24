import { useEffect, useState, type FormEvent } from "react";
import { createPortal } from "react-dom";

import { Button } from "@/components/ui/button";
import { getAssetDetailApiUrl, getAssetsApiUrl, readApiErrorMessage } from "@/lib/api";

import type { AssetResponse } from "@/lib/api";

type AssetFormModalProps = {
  open: boolean;
  editingAsset: AssetResponse | null;
  onClose: () => void;
  onSaved: () => void;
};

type FormState = {
  symbol: string;
  name: string;
  type: string;
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
  const [formState, setFormState] = useState<FormState>(initialFormState);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string[]>>({});
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (!open) {
      return;
    }

    setFormState(
      editingAsset
        ? {
            symbol: editingAsset.symbol,
            name: editingAsset.name,
            type: editingAsset.asset_type,
            quoteSymbol: editingAsset.quote_symbol || "",
            isin: editingAsset.isin || "",
          }
        : initialFormState,
    );
    setFieldErrors({});
    setSubmitError(null);
  }, [editingAsset, open]);

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

  const getFieldError = (field: string) => fieldErrors[field]?.[0] ?? null;

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setFieldErrors({});
    setSubmitError(null);
    setIsSubmitting(true);

    try {
      const payload = {
        symbol: formState.symbol,
        name: formState.name,
        asset_type: formState.type,
        quote_symbol: formState.quoteSymbol || null,
        isin: formState.isin || null,
      };

      const url = editingAsset
        ? getAssetDetailApiUrl(editingAsset.id)
        : getAssetsApiUrl();
      const method = editingAsset ? "PUT" : "POST";

      const response = await fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });

      if (response.status === 422) {
        const data = (await response.json()) as {
          message: string;
          field_errors: Record<string, string[]>;
        };
        setSubmitError(data.message);
        setFieldErrors(data.field_errors ?? {});
        return;
      }

      if (!response.ok) {
        const message = await readApiErrorMessage(
          response,
          editingAsset ? "Failed to update asset" : "Failed to create asset",
        );
        setSubmitError(message);
        return;
      }

      onSaved();
    } catch {
      setSubmitError("Network error");
    } finally {
      setIsSubmitting(false);
    }
  };

  return createPortal(
    <div
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4 animate-in fade-in duration-200"
      role="dialog"
    >
      <div className="flex max-h-full w-full max-w-md flex-col rounded-xl border bg-background shadow-2xl animate-in zoom-in-95 duration-200">
        <header className="flex-none border-b px-6 py-4">
          <h2 className="text-lg font-semibold">
            {editingAsset ? "Edit Asset" : "Add Asset"}
          </h2>
          <p className="text-sm text-muted-foreground">
            {editingAsset
              ? "Update asset details."
              : "Add a new asset to use in transactions."}
          </p>
        </header>
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
                      type: event.target.value,
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
      </div>
    </div>,
    document.body,
  );
}
