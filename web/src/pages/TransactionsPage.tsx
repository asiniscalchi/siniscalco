import { useEffect, useState } from "react";

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
  getAssetTransactionsApiUrl,
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
  const { hideValues } = useUiState();
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Form state
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

        const accountsData = (await accountsRes.json()) as Account[];
        const assetsData = (await assetsRes.json()) as Asset[];

        setAccounts(accountsData);
        setAssets(assetsData);
      } catch {
        if (!cancelled) {
          setError("Failed to load data");
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
    if (!selectedAccountId) {
      setTransactions([]);
      return;
    }

    let cancelled = false;

    async function loadTransactions() {
      try {
        const res = await fetch(getAssetTransactionsApiUrl(selectedAccountId));

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

    // Default currency based on selected account
    const account = accounts.find((a) => String(a.id) === selectedAccountId);
    if (account) {
      setFormCurrency(account.base_currency);
    }

    return () => {
      cancelled = true;
    };
  }, [selectedAccountId, accounts]);

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

      const res = await fetch(getAssetTransactionsApiUrl(), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });

      if (!res.ok) {
        const msg = await readApiErrorMessage(
          res,
          "Failed to create transaction",
        );
        setSubmitError(msg);
        return;
      }

      // Success - reset form
      setFormAssetId("");
      setFormQuantity("");
      setFormUnitPrice("");
      setFormNotes("");

      // Refresh transactions
      const transRes = await fetch(
        getAssetTransactionsApiUrl(selectedAccountId),
      );
      if (transRes.ok) {
        const transData = (await transRes.json()) as Transaction[];
        setTransactions(transData);
      }
    } catch {
      setSubmitError("Network error");
    } finally {
      setIsSubmitting(false);
    }
  };

  if (loading) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 py-8">
        <div className="h-8 w-48 animate-pulse rounded-full bg-muted" />
        <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 py-8">
        <Card className="border-destructive/30 bg-background">
          <CardHeader>
            <CardTitle>Error</CardTitle>
            <CardDescription>{error}</CardDescription>
          </CardHeader>
        </Card>
      </div>
    );
  }

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
        <div className="flex items-center gap-3">
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
            <option value="">Select account...</option>
            {accounts.map((a) => (
              <option key={a.id} value={String(a.id)}>
                {a.name}
              </option>
            ))}
          </select>
        </div>
      </header>

      {!selectedAccountId ? (
        <Card className="border-dashed bg-muted/30">
          <CardHeader className="text-center py-12">
            <CardTitle className="text-lg">No account selected</CardTitle>
            <CardDescription>
              Please select an account from the header to view and add
              transactions.
            </CardDescription>
          </CardHeader>
        </Card>
      ) : (
        <>
          <Card className="bg-background">
            <CardHeader>
              <CardTitle>Add Transaction</CardTitle>
              <CardDescription>
                Record a new BUY or SELL transaction for this account.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <form
                className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-4"
                onSubmit={handleSubmit}
              >
                <div className="flex flex-col gap-1.5">
                  <label
                    className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                    htmlFor="asset-select"
                  >
                    Asset
                  </label>
                  <select
                    required
                    className="rounded-md border bg-background px-3 py-2 text-sm shadow-sm"
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
                    Type
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
                    Trade Date
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
                    Quantity
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
                    Unit Price
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
                    Currency
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
                <div className="flex flex-col gap-1.5 sm:col-span-2 lg:col-span-2">
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
                <div className="flex items-end sm:col-span-2 lg:col-span-4">
                  <Button
                    className="w-full lg:w-auto lg:px-8"
                    disabled={isSubmitting}
                    type="submit"
                  >
                    {isSubmitting ? "Adding..." : "Add Transaction"}
                  </Button>
                </div>
                {submitError && (
                  <div className="col-span-full rounded-md border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                    {submitError}
                  </div>
                )}
              </form>
            </CardContent>
          </Card>

          <Card className="bg-background">
            <CardHeader>
              <CardTitle>Transaction History</CardTitle>
              <CardDescription>
                Recent transactions for the selected account.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {transactions.length === 0 ? (
                <div className="py-8 text-center text-sm text-muted-foreground">
                  No transactions recorded for this account.
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
                        <th className="pb-3">Notes</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y">
                      {transactions.map((t) => {
                        const asset = assets.find((a) => a.id === t.asset_id);
                        return (
                          <tr
                            className="group transition-colors hover:bg-muted/30"
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
                              className="py-3 text-muted-foreground italic truncate max-w-[150px]"
                              title={t.notes || ""}
                            >
                              {t.notes}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              )}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
