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
  getTransactionDetailApiUrl,
  getTransactionsApiUrl,
  readApiErrorMessage,
} from "@/lib/api";
import { useUiState } from "@/lib/ui-state";

import { TransactionFormModal } from "./TransactionFormModal";
import { TransactionsHistoryCard } from "./TransactionsHistoryCard";
import type { Account, Asset, Transaction } from "./types";

export function TransactionsPage() {
  const [isLocked, setIsLocked] = useState(true);
  const { hideValues } = useUiState();
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [initialDataError, setInitialDataError] = useState<string | null>(null);
  const [transactionsError, setTransactionsError] = useState<string | null>(null);
  const [retryToken, setRetryToken] = useState(0);

  const [showModal, setShowModal] = useState(false);
  const [editingTransactionId, setEditingTransactionId] = useState<number | null>(null);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadInitialData() {
      setLoading(true);
      setInitialDataError(null);

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
          setInitialDataError("Failed to load initial data");
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
  }, [retryToken]);

  useEffect(() => {
    let cancelled = false;

    async function loadTransactions() {
      setTransactionsError(null);

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
          setTransactionsError("Failed to load transactions");
        }
      }
    }

    void loadTransactions();

    return () => {
      cancelled = true;
    };
  }, [selectedAccountId, retryToken]);

  const handleCreateClick = () => {
    setEditingTransactionId(null);
    setShowModal(true);
  };

  const handleEditClick = (t: Transaction) => {
    setEditingTransactionId(t.id);
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

  const handleModalSaved = () => {
    setShowModal(false);
    setEditingTransactionId(null);
    setRetryToken((t) => t + 1);
  };

  if (loading && transactions.length === 0 && accounts.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
        <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />
      </div>
    );
  }

  const pageError = initialDataError ?? transactionsError;

  if (pageError && transactions.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 py-8">
        <Card className="border-destructive/30 bg-background">
          <CardHeader>
            <CardTitle>Error</CardTitle>
            <CardDescription>{pageError}</CardDescription>
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
      <TransactionsHistoryCard
        accounts={accounts}
        assets={assets}
        editingTransactionId={editingTransactionId}
        hideValues={hideValues}
        isDeleting={isDeleting}
        isLocked={isLocked}
        onAccountChange={setSelectedAccountId}
        onCreateClick={handleCreateClick}
        onDeleteClick={handleDeleteClick}
        onEditClick={handleEditClick}
        onToggleLock={() => setIsLocked((locked) => !locked)}
        selectedAccountId={selectedAccountId}
        transactions={transactions}
      />

      <TransactionFormModal
        accounts={accounts}
        assets={assets}
        editingTransaction={editingTransactionId
          ? transactions.find((transaction) => transaction.id === editingTransactionId) ?? null
          : null}
        onClose={() => {
          setShowModal(false);
          setEditingTransactionId(null);
        }}
        onSaved={handleModalSaved}
        open={showModal}
        selectedAccountId={selectedAccountId}
      />
    </div>
  );
}
