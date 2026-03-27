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
  fetchAccounts,
  fetchAssets,
  fetchTransactions,
  deleteTransaction,
  extractGqlErrorMessage,
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
        const [accountsData, assetsData] = await Promise.all([
          fetchAccounts(),
          fetchAssets(),
        ]);

        if (cancelled) return;

        setAccounts(accountsData.map((a) => ({
          id: a.id,
          name: a.name,
          accountType: a.accountType,
          baseCurrency: a.baseCurrency,
        })));
        setAssets(assetsData.map((a) => ({
          id: a.id,
          symbol: a.symbol,
          name: a.name,
          assetType: a.assetType,
          isin: a.isin,
        })));
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
        const accountId = selectedAccountId ? parseInt(selectedAccountId) : undefined;
        const data = await fetchTransactions(accountId);

        if (cancelled) return;

        setTransactions(data.map((t) => ({
          id: t.id,
          accountId: t.accountId,
          assetId: t.assetId,
          transactionType: t.transactionType as "BUY" | "SELL",
          tradeDate: t.tradeDate,
          quantity: t.quantity,
          unitPrice: t.unitPrice,
          currencyCode: t.currencyCode,
          notes: t.notes,
        })));
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
      await deleteTransaction(transactionId);
      setRetryToken((t) => t + 1);
    } catch (error) {
      alert(extractGqlErrorMessage(error, "Failed to delete transaction"));
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
