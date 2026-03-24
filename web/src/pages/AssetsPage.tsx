import { useEffect, useState } from "react";
import { createPortal } from "react-dom";

import {
  LockIcon,
  PencilIcon,
  PlusIcon,
  TrashIcon,
  UnlockIcon,
} from "@/components/Icons";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  getAssetDetailApiUrl,
  getAssetsApiUrl,
  readApiErrorMessage,
  type AssetResponse,
} from "@/lib/api";
import { cn } from "@/lib/utils";

export function AssetsPage() {
  const [isLocked, setIsLocked] = useState(true);
  const [assets, setAssets] = useState<AssetResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryToken, setRetryToken] = useState(0);

  // Modal state
  const [showModal, setShowModal] = useState(false);
  const [editingAsset, setEditingAsset] = useState<AssetResponse | null>(null);
  const [formSymbol, setFormSymbol] = useState("");
  const [formName, setFormName] = useState("");
  const [formType, setFormType] = useState("STOCK");
  const [formIsin, setFormIsin] = useState("");
  const [fieldErrors, setFieldErrors] = useState<Record<string, string[]>>({});
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadAssets() {
      setLoading(true);
      setError(null);

      try {
        const response = await fetch(getAssetsApiUrl());

        if (!response.ok) {
          throw new Error("Failed to load assets");
        }

        const data = (await response.json()) as AssetResponse[];

        if (!cancelled) {
          setAssets(data);
        }
      } catch {
        if (!cancelled) {
          setError("Failed to load assets");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadAssets();

    return () => {
      cancelled = true;
    };
  }, [retryToken]);

  const resetForm = () => {
    setEditingAsset(null);
    setFormSymbol("");
    setFormName("");
    setFormType("STOCK");
    setFormIsin("");
    setFieldErrors({});
    setSubmitError(null);
  };

  const handleCreateClick = () => {
    resetForm();
    setShowModal(true);
  };

  const handleEditClick = (asset: AssetResponse) => {
    setEditingAsset(asset);
    setFormSymbol(asset.symbol);
    setFormName(asset.name);
    setFormType(asset.asset_type);
    setFormIsin(asset.isin || "");
    setFieldErrors({});
    setSubmitError(null);
    setShowModal(true);
  };

  const handleDeleteClick = async (asset: AssetResponse) => {
    if (!window.confirm(`Are you sure you want to delete ${asset.symbol}?`)) {
      return;
    }

    setIsDeleting(asset.id);
    try {
      const response = await fetch(getAssetDetailApiUrl(asset.id), {
        method: "DELETE",
      });

      if (!response.ok) {
        const message = await readApiErrorMessage(
          response,
          "Failed to delete asset",
        );
        alert(message);
        return;
      }

      setRetryToken((t) => t + 1);
    } catch {
      alert("Network error while deleting asset");
    } finally {
      setIsDeleting(null);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFieldErrors({});
    setSubmitError(null);
    setIsSubmitting(true);

    try {
      const payload = {
        symbol: formSymbol,
        name: formName,
        asset_type: formType,
        isin: formIsin || null,
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

      setShowModal(false);
      setRetryToken((t) => t + 1);
    } catch {
      setSubmitError("Network error");
    } finally {
      setIsSubmitting(false);
    }
  };

  const getFieldError = (field: string) => fieldErrors[field]?.[0] ?? null;

  if (loading && assets.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
        <header className="flex flex-col gap-4 rounded-xl border bg-background px-6 py-5 shadow-sm sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-1">
            <h1 className="text-2xl font-semibold tracking-tight">Assets</h1>
            <p className="max-w-2xl text-sm text-muted-foreground">
              Manage the assets you use in transactions.
            </p>
          </div>
          <div className="h-10 w-32 animate-pulse rounded-lg bg-muted" />
        </header>
        <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />
      </div>
    );
  }

  if (error && assets.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 py-8">
        <Card className="border-destructive/30 bg-background">
          <CardHeader>
            <CardTitle>Error</CardTitle>
            <CardDescription>{error}</CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => setRetryToken((t) => t + 1)}>Retry</Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="mx-auto flex min-w-0 w-full max-w-4xl flex-col gap-6 overflow-x-hidden">
      <header className="flex flex-col gap-4 rounded-xl border bg-background px-6 py-5 shadow-sm sm:flex-row sm:items-center sm:justify-between">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">Assets</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            Manage the assets you use in transactions.
          </p>
        </div>
        <div className="flex items-center justify-end gap-2">
          <Button
            aria-label={isLocked ? "Unlock edit mode" : "Lock edit mode"}
            className={cn(
              "size-9 rounded-full transition-colors",
              !isLocked &&
                "bg-amber-100 text-amber-900 hover:bg-amber-200 dark:bg-amber-900/30 dark:text-amber-400 dark:hover:bg-amber-900/50",
            )}
            onClick={() => setIsLocked(!isLocked)}
            size="icon"
            type="button"
            variant="ghost"
          >
            {isLocked ? <LockIcon /> : <UnlockIcon />}
          </Button>
          <Button
            aria-label="Add Asset"
            onClick={handleCreateClick}
            size="icon-lg"
            title="Add Asset"
          >
            <PlusIcon />
          </Button>
        </div>
      </header>

      <Card className="min-w-0 bg-background">
        <CardContent className="min-w-0 pt-6">
          {assets.length === 0 ? (
            <div className="py-12 text-center">
              <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-full bg-muted">
                <PlusIcon className="size-6 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium">No assets yet</h3>
              <p className="mb-6 text-sm text-muted-foreground">
                Add your first asset to start recording transactions.
              </p>
              <Button
                aria-label="Add Asset"
                onClick={handleCreateClick}
                size="icon-lg"
                title="Add Asset"
                variant="outline"
              >
                <PlusIcon />
              </Button>
            </div>
          ) : (
            <div className="w-full overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b text-left font-semibold text-muted-foreground uppercase tracking-wider text-[11px]">
                    <th className="pb-3 pr-4">Symbol</th>
                    <th className="pb-3 pr-4">Name</th>
                    <th className="pb-3 pr-4">Type</th>
                    <th className="pb-3 pr-4">ISIN</th>
                    {!isLocked && <th className="pb-3 text-right">Actions</th>}
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {assets.map((asset) => (
                    <tr
                      className="group transition-colors hover:bg-muted/30"
                      key={asset.id}
                    >
                      <td className="py-3 pr-4 font-bold tabular-nums">
                        {asset.symbol}
                      </td>
                      <td className="py-3 pr-4">{asset.name}</td>
                      <td className="py-3 pr-4">
                        <span className="inline-flex items-center rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide">
                          {asset.asset_type.replace("_", " ")}
                        </span>
                      </td>
                      <td className="py-3 pr-4 font-mono text-[11px] text-muted-foreground">
                        {asset.isin || "—"}
                      </td>
                      {!isLocked && (
                        <td className="py-3 text-right">
                          <div className="flex justify-end gap-1">
                            <Button
                              disabled={isDeleting !== null}
                              onClick={() => handleEditClick(asset)}
                              size="icon"
                              title="Edit asset"
                              variant="ghost"
                            >
                              <PencilIcon />
                              <span className="sr-only">Edit</span>
                            </Button>
                            <Button
                              className="text-destructive hover:bg-destructive/10"
                              disabled={isDeleting !== null}
                              onClick={() => handleDeleteClick(asset)}
                              size="icon"
                              title="Delete asset"
                              variant="ghost"
                            >
                              {isDeleting === asset.id ? (
                                <div className="size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
                              ) : (
                                <TrashIcon />
                              )}
                              <span className="sr-only">Delete</span>
                            </Button>
                          </div>
                        </td>
                      )}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create/Edit Modal */}
      {showModal &&
        createPortal(
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
              <form
                className="flex flex-1 flex-col overflow-hidden"
                onSubmit={handleSubmit}
              >
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
                        onChange={(e) => setFormSymbol(e.target.value)}
                        placeholder="AAPL"
                        value={formSymbol}
                      />
                      {getFieldError("symbol") ? (
                        <p className="text-xs text-destructive">
                          {getFieldError("symbol")}
                        </p>
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
                        onChange={(e) => setFormName(e.target.value)}
                        placeholder="Apple Inc."
                        value={formName}
                      />
                      {getFieldError("name") ? (
                        <p className="text-xs text-destructive">
                          {getFieldError("name")}
                        </p>
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
                        onChange={(e) => setFormType(e.target.value)}
                        value={formType}
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
                        htmlFor="asset-isin"
                      >
                        ISIN
                      </label>
                      <input
                        className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
                        id="asset-isin"
                        onChange={(e) => setFormIsin(e.target.value)}
                        placeholder="US0378331005"
                        value={formIsin}
                      />
                    </div>
                  </div>
                  {submitError ? (
                    <div className="rounded-md border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                      {submitError}
                    </div>
                  ) : null}
                </div>
                <footer className="flex flex-none justify-end gap-3 border-t bg-muted/30 px-6 py-4 rounded-b-xl">
                  <Button
                    onClick={() => setShowModal(false)}
                    type="button"
                    variant="outline"
                  >
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
        )}
    </div>
  );
}
