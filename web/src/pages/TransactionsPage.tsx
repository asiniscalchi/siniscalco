import { useEffect, useState } from "react";

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
  getAccountsApiUrl,
  getAssetsApiUrl,
  getTransactionDetailApiUrl,
  getTransactionsApiUrl,
  readApiErrorMessage,
} from "@/lib/api";
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

type Account = {
  id: number;
  name: string;
  account_type: string;
  base_currency: string;
};

type Asset = {
  id: number;
  symbol: string;
  name: string;
  asset_type: string;
  isin?: string | null;
};

type Transaction = {
  id: number;
  account_id: number;
  asset_id: number;
  transaction_type: "BUY" | "SELL";
  trade_date: string;
  quantity: string;
  unit_price: string;
  currency_code: string;
  notes: string | null;
};

export function TransactionsPage() {
  const [isLocked, setIsLocked] = useState(true);
  const { hideValues } = useUiState();
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryToken, setRetryToken] = useState(0);

  // Modal/Form state
  const [showModal, setShowModal] = useState(false);
  const [editingTransactionId, setEditingTransactionId] = useState<number | null>(null);
  const [formAssetId, setFormAssetId] = useState("");
  const [formType, setFormType] = useState<"BUY" | "SELL">("BUY");
  const [formTradeDate, setFormTradeDate] = useState(
    new Date().toISOString().split("T")[0],
  );
  const [formQuantity, setFormQuantity] = useState("");
  const [formUnitPrice, setFormUnitPrice] = useState("");
  const [formCurrency, setFormCurrency] = useState("");
  const [formNotes, setFormNotes] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  const transactionSubmitDisabled = isSubmitting || assets.length === 0 || !formAssetId;

  useEffect(() => {
    let cancelled = false;

    async function loadInitialData() {
      try {
        const [accountsRes, assetsRes] = await Promise.all([
          fetch(getAccountsApiUrl()),
          fetch(getAssetsApiUrl()),
        ]);

        if (cancelled) return;

        if (!accountsRes.ok || !assetsRes.ok) {
          throw new Error("Failed to load initial data");
        }

        const [accountsData, assetsData] = await Promise.all([
          accountsRes.json() as Promise<Account[]>,
          assetsRes.json() as Promise<Asset[]>,
        ]);

        setAccounts(accountsData);
        setAssets(assetsData);
      } catch {
        if (!cancelled) {
          setError("Failed to load initial data");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadInitialData();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function loadTransactions() {
      try {
        const res = await fetch(getTransactionsApiUrl(selectedAccountId || undefined));

        if (cancelled) return;

        if (!res.ok) {
          throw new Error("Failed to load transactions");
        }

        const data = (await res.json()) as Transaction[];
        setTransactions(data);
      } catch {
        if (!cancelled) {
          setError("Failed to load transactions");
        }
      }
    }

    void loadTransactions();

    return () => {
      cancelled = true;
    };
  }, [selectedAccountId, retryToken]);

  const resetForm = () => {
    setEditingTransactionId(null);
    setFormAssetId("");
    setFormQuantity("");
    setFormUnitPrice("");
    setFormNotes("");
    setFormType("BUY");
    setFormTradeDate(new Date().toISOString().split("T")[0]);
    setSubmitError(null);

    // Default currency based on selected account
    if (selectedAccountId) {
      const account = accounts.find((a) => String(a.id) === selectedAccountId);
      if (account) {
        setFormCurrency(account.base_currency);
      }
    } else {
      setFormCurrency("");
    }
  };

  const handleCreateClick = () => {
    resetForm();
    setShowModal(true);
  };

  const handleEditClick = (t: Transaction) => {
    setEditingTransactionId(t.id);
    setFormAssetId(String(t.asset_id));
    setFormType(t.transaction_type);
    setFormTradeDate(t.trade_date);
    setFormQuantity(t.quantity);
    setFormUnitPrice(t.unit_price);
    setFormCurrency(t.currency_code);
    setFormNotes(t.notes || "");
    setSubmitError(null);
    setShowModal(true);
  };

  const handleDeleteClick = async (transactionId: number) => {
    if (!window.confirm("Are you sure you want to delete this transaction?")) {
      return;
    }

    setIsDeleting(transactionId);
    try {
      const res = await fetch(getTransactionDetailApiUrl(transactionId), {
        method: "DELETE",
      });

      if (!res.ok) {
        const msg = await readApiErrorMessage(res, "Failed to delete transaction");
        alert(msg);
        return;
      }

      setRetryToken((t) => t + 1);
    } catch {
      alert("Network error while deleting transaction");
    } finally {
      setIsDeleting(null);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitError(null);
    setIsSubmitting(true);

    try {
      const payload = {
        account_id: parseInt(selectedAccountId),
        asset_id: parseInt(formAssetId),
        transaction_type: formType,
        trade_date: formTradeDate,
        quantity: formQuantity,
        unit_price: formUnitPrice,
        currency_code: formCurrency,
        notes: formNotes || null,
      };

      const url = editingTransactionId
        ? getTransactionDetailApiUrl(editingTransactionId)
        : getTransactionsApiUrl();
      const method = editingTransactionId ? "PUT" : "POST";

      const res = await fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });

      if (!res.ok) {
        const msg = await readApiErrorMessage(
          res,
          editingTransactionId
            ? "Failed to update transaction"
            : "Failed to create transaction",
        );
        setSubmitError(msg);
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

  if (loading && transactions.length === 0 && accounts.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 py-8">
        <div className="h-8 w-48 animate-pulse rounded-full bg-muted" />
        <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />
      </div>
    );
  }

  if (error && transactions.length === 0) {
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

  const transactionHistoryCard = (
    <Card className="bg-background">
      <CardHeader>
        <CardTitle>Transaction History</CardTitle>
        <CardDescription>
          {selectedAccountId
            ? `Recent transactions for ${accounts.find(a => String(a.id) === selectedAccountId)?.name || "selected account"}.`
            : "Showing all recorded transactions."}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {transactions.length === 0 ? (
          <div className="py-12 text-center text-sm text-muted-foreground">
            No transactions recorded.
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left font-semibold text-muted-foreground uppercase tracking-wider text-[11px]">
                  <th className="pb-3 pr-4">Date</th>
                  <th className="pb-3 pr-4">Asset</th>
                  <th className="pb-3 pr-4">Type</th>
                  <th className="pb-3 pr-4 text-right">Quantity</th>
                  <th className="pb-3 pr-4 text-right">Price</th>
                  <th className="pb-3 pr-4">Curr</th>
                  <th className="pb-3 pr-4">Notes</th>
                  {!isLocked && <th className="pb-3 text-right">Actions</th>}
                </tr>
              </thead>
              <tbody className="divide-y">
                {transactions.map((t) => {
                  const asset = assets.find((a) => a.id === t.asset_id);
                  return (
                    <tr
                      className={cn(
                        "group transition-colors hover:bg-muted/30",
                        editingTransactionId === t.id && "bg-muted/50",
                      )}
                      key={t.id}
                    >
                      <td className="py-3 pr-4 whitespace-nowrap tabular-nums">
                        {t.trade_date}
                      </td>
                      <td className="py-3 pr-4">
                        <div className="flex flex-col">
                          <span className="font-bold">
                            {asset?.symbol || "Unknown"}
                          </span>
                          <span className="text-[10px] text-muted-foreground truncate max-w-[120px]">
                            {asset?.name}
                          </span>
                        </div>
                      </td>
                      <td className="py-3 pr-4">
                        <span
                          className={cn(
                            "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide",
                            t.transaction_type === "BUY"
                              ? "bg-emerald-50 text-emerald-700 border border-emerald-200"
                              : "bg-amber-50 text-amber-700 border border-amber-200",
                          )}
                        >
                          {t.transaction_type}
                        </span>
                      </td>
                      <td className="py-3 pr-4 text-right font-mono tabular-nums">
                        {t.quantity}
                      </td>
                      <td className="py-3 pr-4 text-right">
                        <MoneyText
                          className="text-right"
                          hidden={hideValues}
                          includeCurrency={false}
                          value={t.unit_price}
                        />
                      </td>
                      <td className="py-3 pr-4 font-mono text-muted-foreground text-[11px]">
                        {t.currency_code}
                      </td>
                      <td
                        className="py-3 pr-4 text-muted-foreground italic truncate max-w-[150px]"
                        title={t.notes || ""}
                      >
                        {t.notes}
                      </td>
                      {!isLocked && (
                        <td className="py-3 text-right">
                          <div className="flex justify-end gap-1">
                            <Button
                              disabled={isDeleting !== null}
                              onClick={() => handleEditClick(t)}
                              size="icon"
                              title="Edit transaction"
                              type="button"
                              variant="ghost"
                            >
                              <PencilIcon />
                              <span className="sr-only">Edit</span>
                            </Button>
                            <Button
                              className="text-destructive hover:bg-destructive/10"
                              disabled={isDeleting !== null}
                              onClick={() => handleDeleteClick(t.id)}
                              size="icon"
                              title="Delete transaction"
                              type="button"
                              variant="ghost"
                            >
                              {isDeleting === t.id ? (
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
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </CardContent>
    </Card>
  );

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
      <header className="flex flex-col gap-4 rounded-xl border bg-background px-6 py-5 shadow-sm sm:flex-row sm:items-center sm:justify-between">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">
            Transactions
          </h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            Manage your asset transactions.
          </p>
        </div>
        <div className="flex flex-col gap-4 items-end sm:flex-row sm:items-center">
          <div className="flex items-center justify-end gap-3">
            <label
              className="text-sm font-medium text-muted-foreground"
              htmlFor="account-selector"
            >
              Account:
            </label>
            <select
              className="rounded-md border bg-background px-3 py-1.5 text-sm shadow-sm transition-colors focus:outline-hidden focus:ring-1 focus:ring-ring"
              id="account-selector"
              onChange={(e) => setSelectedAccountId(e.target.value)}
              value={selectedAccountId}
            >
              <option value="">All Accounts</option>
              {accounts.map((a) => (
                <option key={a.id} value={String(a.id)}>
                  {a.name}
                </option>
              ))}
            </select>
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
              aria-label="Add Transaction"
              disabled={!selectedAccountId}
              onClick={handleCreateClick}
              size="icon-lg"
              title="Add Transaction"
            >
              <PlusIcon />
            </Button>
          </div>
        </div>
      </header>

      {transactionHistoryCard}

      {/* Create/Edit Transaction Modal */}
      {showModal && (
        <div
          aria-modal="true"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4 animate-in fade-in duration-200"
          role="dialog"
        >
          <div className="w-full max-w-2xl rounded-xl border bg-background shadow-2xl animate-in zoom-in-95 duration-200">
            <header className="border-b px-6 py-4">
              <h2 className="text-lg font-semibold">
                {editingTransactionId ? "Edit Transaction" : "Add Transaction"}
              </h2>
              <p className="text-sm text-muted-foreground">
                {editingTransactionId
                  ? "Update transaction details."
                  : `Record a new transaction for ${accounts.find(a => String(a.id) === selectedAccountId)?.name}.`}
              </p>
            </header>
            <form onSubmit={handleSubmit}>
              <div className="grid gap-5 px-6 py-6 sm:grid-cols-2">
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
                    onChange={(e) => setFormAssetId(e.target.value)}
                    value={formAssetId}
                  >
                    <option value="">Select asset...</option>
                    {assets.map((a) => (
                      <option key={a.id} value={String(a.id)}>
                        {a.symbol} — {a.name}
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
                    onChange={(e) =>
                      setFormType(e.target.value as "BUY" | "SELL")
                    }
                    value={formType}
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
                    onChange={(e) => setFormTradeDate(e.target.value)}
                    type="date"
                    value={formTradeDate}
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
                    className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm font-mono"
                    id="quantity-input"
                    min="0.00000001"
                    onChange={(e) => setFormQuantity(e.target.value)}
                    placeholder="0.00"
                    step="any"
                    type="number"
                    value={formQuantity}
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
                    className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm font-mono"
                    id="price-input"
                    min="0"
                    onChange={(e) => setFormUnitPrice(e.target.value)}
                    placeholder="0.00"
                    step="any"
                    type="number"
                    value={formUnitPrice}
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
                    className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm font-mono uppercase"
                    id="currency-input"
                    maxLength={3}
                    onChange={(e) => setFormCurrency(e.target.value)}
                    placeholder="USD"
                    type="text"
                    value={formCurrency}
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
                    onChange={(e) => setFormNotes(e.target.value)}
                    placeholder="Optional notes"
                    type="text"
                    value={formNotes}
                  />
                </div>
                {submitError && (
                  <div className="col-span-full rounded-md border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                    {submitError}
                  </div>
                )}
              </div>
              <footer className="flex justify-end gap-3 border-t bg-muted/30 px-6 py-4 rounded-b-xl">
                <Button
                  onClick={() => setShowModal(false)}
                  type="button"
                  variant="outline"
                >
                  Cancel
                </Button>
                <Button disabled={transactionSubmitDisabled} type="submit">
                  {isSubmitting
                    ? editingTransactionId
                      ? "Saving..."
                      : "Adding..."
                    : editingTransactionId
                      ? "Save Changes"
                      : "Add Transaction"}
                </Button>
              </footer>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

