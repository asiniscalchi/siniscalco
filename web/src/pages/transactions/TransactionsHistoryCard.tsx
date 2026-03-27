import { useState } from "react";
import { gql } from "@apollo/client/core";
import { useMutation, useQuery } from "@apollo/client/react";

import { LockIcon, PlusIcon, UnlockIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { extractGqlErrorMessage } from "@/lib/gql";

const ACCOUNTS_QUERY = gql`
  {
    accounts {
      id name accountType baseCurrency
    }
  }
`;

const ASSETS_QUERY = gql`
  {
    assets {
      id symbol name assetType
    }
  }
`;

const TRANSACTIONS_QUERY = gql`
  query Transactions($accountId: Int) {
    transactions(accountId: $accountId) {
      id accountId assetId transactionType tradeDate
      quantity unitPrice currencyCode notes
    }
  }
`;

const DELETE_TRANSACTION_MUTATION = gql`
  mutation DeleteTransaction($id: Int!) {
    deleteTransaction(id: $id)
  }
`;
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

import { TransactionFormModal } from "./TransactionFormModal";
import { TransactionsHistoryCardDesktopRow } from "./TransactionsHistoryCardDesktopRow";
import { TransactionsHistoryCardEmptyState } from "./TransactionsHistoryCardEmptyState";
import { TransactionsHistoryCardMobileItem } from "./TransactionsHistoryCardMobileItem";
import type { Account, Asset, Transaction } from "./types";

export function TransactionsHistoryCard() {
  const { hideValues } = useUiState();
  const [isLocked, setIsLocked] = useState(true);
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [editingTransactionId, setEditingTransactionId] = useState<number | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  const accountIdVar = selectedAccountId ? parseInt(selectedAccountId) : null;

  const { data: accountsData, loading: accountsLoading, error: accountsError, refetch: refetchAccounts } =
    useQuery<{ accounts: Account[] }>(ACCOUNTS_QUERY);
  const { data: assetsData, loading: assetsLoading, error: assetsError, refetch: refetchAssets } =
    useQuery<{ assets: Asset[] }>(ASSETS_QUERY);
  const { data: transactionsData, error: transactionsError, refetch: refetchTransactions } =
    useQuery<{ transactions: Transaction[] }>(TRANSACTIONS_QUERY, { variables: { accountId: accountIdVar } });

  const [deleteTransactionMutation] = useMutation(DELETE_TRANSACTION_MUTATION);

  const accounts = accountsData?.accounts ?? [];
  const assets = assetsData?.assets ?? [];
  const transactions = transactionsData?.transactions ?? [];
  const assetById = new Map(assets.map((asset) => [asset.id, asset]));

  const loading = accountsLoading || assetsLoading;
  const initialDataError = accountsError ?? assetsError;
  const pageError = initialDataError ?? transactionsError;
  const pageErrorMessage = initialDataError ? "Failed to load initial data" : "Failed to load transactions";

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
    return <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />;
  }

  if (pageError && transactions.length === 0) {
    return (
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
    );
  }

  return (
    <>
      <Card className="min-w-0 bg-background">
        <CardHeader className="pb-2">
          <div className="flex items-center gap-2">
            <h1 className="flex-1 text-2xl font-semibold tracking-tight">Transactions</h1>
            <label className="sr-only" htmlFor="account-selector">Account:</label>
            <select
              className="hidden rounded-md border bg-background px-3 py-1.5 text-sm shadow-sm transition-colors focus:outline-hidden focus:ring-1 focus:ring-ring sm:block"
              id="account-selector"
              onChange={(event) => setSelectedAccountId(event.target.value)}
              value={selectedAccountId}
            >
              <option value="">All Accounts</option>
              {accounts.map((account) => (
                <option key={account.id} value={String(account.id)}>
                  {account.name}
                </option>
              ))}
            </select>
            <Button
              aria-label={isLocked ? "Unlock edit mode" : "Lock edit mode"}
              className={cn(
                "size-9 rounded-full transition-colors",
                !isLocked &&
                  "bg-amber-100 text-amber-900 hover:bg-amber-200 dark:bg-amber-900/30 dark:text-amber-400 dark:hover:bg-amber-900/50",
              )}
              onClick={() => setIsLocked((locked) => !locked)}
              size="icon"
              title={isLocked ? "Unlock edit mode" : "Lock edit mode"}
              type="button"
              variant="ghost"
            >
              {isLocked ? <LockIcon /> : <UnlockIcon />}
            </Button>
            <Button
              aria-label="Add Transaction"
              disabled={!selectedAccountId}
              onClick={() => {
                setEditingTransactionId(null);
                setShowModal(true);
              }}
              size="icon-lg"
              title="Add Transaction"
            >
              <PlusIcon />
            </Button>
          </div>
          <div className="flex justify-end sm:hidden">
            <select
              aria-label="Account"
              className="rounded-md border bg-background px-3 py-1.5 text-sm shadow-sm transition-colors focus:outline-hidden focus:ring-1 focus:ring-ring"
              onChange={(event) => setSelectedAccountId(event.target.value)}
              value={selectedAccountId}
            >
              <option value="">All Accounts</option>
              {accounts.map((account) => (
                <option key={account.id} value={String(account.id)}>
                  {account.name}
                </option>
              ))}
            </select>
          </div>
        </CardHeader>
        <CardContent className="min-w-0 pt-4">
          {transactions.length === 0 ? (
            <TransactionsHistoryCardEmptyState />
          ) : (
            <>
              <div className="space-y-2 sm:hidden">
                {transactions.map((transaction) => (
                  <TransactionsHistoryCardMobileItem
                    asset={assetById.get(transaction.assetId)}
                    hideValues={hideValues}
                    isDeleting={isDeleting}
                    isEditing={editingTransactionId === transaction.id}
                    isLocked={isLocked}
                    key={transaction.id}
                    onDeleteClick={handleDeleteClick}
                    onEditClick={(t) => {
                      setEditingTransactionId(t.id);
                      setShowModal(true);
                    }}
                    transaction={transaction}
                  />
                ))}
              </div>

              <div className="hidden w-full overflow-x-auto sm:block">
                <table className="w-full table-fixed text-sm">
                  <thead>
                    <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                      <th className="w-[100px] pb-3 pr-4">Date</th>
                      <th className="pb-3 pr-4">Asset</th>
                      <th className="w-[80px] pb-3 pr-4">Type</th>
                      <th className="w-[100px] pb-3 pr-4 text-right">Quantity</th>
                      <th className="w-[100px] pb-3 pr-4 text-right">Price</th>
                      <th className="w-[60px] pb-3 pr-4">Curr</th>
                      <th className="pb-3 pr-4">Notes</th>
                      {!isLocked ? <th className="w-[90px] pb-3 text-right">Actions</th> : null}
                    </tr>
                  </thead>
                  <tbody className="divide-y">
                    {transactions.map((transaction) => (
                      <TransactionsHistoryCardDesktopRow
                        asset={assetById.get(transaction.assetId)}
                        hideValues={hideValues}
                        isDeleting={isDeleting}
                        isEditing={editingTransactionId === transaction.id}
                        isLocked={isLocked}
                        key={transaction.id}
                        onDeleteClick={handleDeleteClick}
                        onEditClick={(t) => {
                          setEditingTransactionId(t.id);
                          setShowModal(true);
                        }}
                        transaction={transaction}
                      />
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <TransactionFormModal
        key={showModal ? (editingTransactionId ?? "new") : "closed"}
        accounts={accounts}
        assets={assets}
        editingTransaction={editingTransactionId
          ? transactions.find((t) => t.id === editingTransactionId) ?? null
          : null}
        onClose={() => {
          setShowModal(false);
          setEditingTransactionId(null);
        }}
        onSaved={handleModalSaved}
        open={showModal}
        selectedAccountId={selectedAccountId}
      />
    </>
  );
}
