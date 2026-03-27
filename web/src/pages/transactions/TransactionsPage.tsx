import { useState } from "react";
import { useMutation, useQuery } from "@apollo/client/react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  ACCOUNTS_QUERY,
  ASSETS_QUERY,
  DELETE_TRANSACTION_MUTATION,
  TRANSACTIONS_QUERY,
  extractGqlErrorMessage,
} from "@/lib/api";
import { useUiState } from "@/lib/ui-state";

import { TransactionFormModal } from "./TransactionFormModal";
import { TransactionsHistoryCard } from "./TransactionsHistoryCard";
import type { Account, Asset, Transaction } from "./types";

export function TransactionsPage() {
  const [isLocked, setIsLocked] = useState(true);
  const { hideValues } = useUiState();
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [editingTransactionId, setEditingTransactionId] = useState<number | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  const accountIdVar = selectedAccountId ? parseInt(selectedAccountId) : null;

  const { data: accountsData, loading: accountsLoading, error: accountsError, refetch: refetchAccounts } = useQuery<{ accounts: Account[] }>(ACCOUNTS_QUERY);
  const { data: assetsData, loading: assetsLoading, error: assetsError, refetch: refetchAssets } = useQuery<{ assets: Asset[] }>(ASSETS_QUERY);
  const { data: transactionsData, error: transactionsError, refetch: refetchTransactions } = useQuery<{ transactions: Transaction[] }>(
    TRANSACTIONS_QUERY,
    { variables: { accountId: accountIdVar } },
  );

  const [deleteTransactionMutation] = useMutation(DELETE_TRANSACTION_MUTATION);

  const accounts = accountsData?.accounts ?? [];
  const assets = assetsData?.assets ?? [];
  const transactions = transactionsData?.transactions ?? [];

  const loading = accountsLoading || assetsLoading;
  const initialDataError = accountsError ?? assetsError;
  const pageError = initialDataError ?? transactionsError;
  const pageErrorMessage = initialDataError ? "Failed to load initial data" : "Failed to load transactions";

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
      await deleteTransactionMutation({ variables: { id: transactionId } });
      await refetchTransactions();
    } catch (error) {
      alert(extractGqlErrorMessage(error, "Failed to delete transaction"));
    } finally {
      setIsDeleting(null);
    }
  };

  const handleModalSaved = () => {
    setShowModal(false);
    setEditingTransactionId(null);
    void refetchTransactions();
  };

  if (loading && transactions.length === 0 && accounts.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
        <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />
      </div>
    );
  }

  if (pageError && transactions.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 py-8">
        <Card className="border-destructive/30 bg-background">
          <CardHeader>
            <CardTitle>Error</CardTitle>
            <CardDescription>{pageErrorMessage}</CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => {
              void refetchAccounts();
              void refetchAssets();
              void refetchTransactions();
            }}>
              Retry
            </Button>
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
        key={showModal ? (editingTransactionId ?? "new") : "closed"}
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
