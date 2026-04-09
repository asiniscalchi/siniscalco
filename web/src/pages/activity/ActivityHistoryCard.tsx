import { useState } from "react";
import { gql } from "@apollo/client/core";
import { useMutation, useQuery } from "@apollo/client/react";

import { LockIcon, PlusIcon, UnlockIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { extractGqlErrorMessage } from "@/lib/gql";
import { type ActivityCashMovementsQuery, type ActivityTransfersQuery, type TransactionAccountsQuery, type TransactionAssetsQuery, type TransactionsQuery } from "@/gql/types";

const ACCOUNTS_QUERY = gql`
  query TransactionAccounts {
    accounts {
      id name accountType baseCurrency
    }
  }
`;

const ASSETS_QUERY = gql`
  query TransactionAssets {
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

const CASH_MOVEMENTS_QUERY = gql`
  query ActivityCashMovements($accountId: Int) {
    cashMovements(accountId: $accountId) {
      id accountId currency amount date notes
    }
  }
`;

const TRANSFERS_QUERY = gql`
  query ActivityTransfers($accountId: Int) {
    transfers(accountId: $accountId) {
      id fromAccountId toAccountId
      fromCurrency fromAmount toCurrency toAmount
      transferDate notes
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

import { ActivityActionPickerModal, type ActivityCreateAction } from "./ActivityActionPickerModal";
import { TransactionFormModal } from "./TransactionFormModal";
import { CashMovementFormModal } from "./CashMovementFormModal";
import { ActivityHistoryCardDesktopRow } from "./ActivityHistoryCardDesktopRow";
import { ActivityHistoryCardEmptyState } from "./ActivityHistoryCardEmptyState";
import { ActivityHistoryCardMobileItem } from "./ActivityHistoryCardMobileItem";
import { TransferFormModal } from "../transfers/TransferFormModal";
import type { ActivityFilter, ActivityItem } from "./types";

const FILTER_LABELS: { value: ActivityFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "trades", label: "Trades" },
  { value: "cash", label: "Cash" },
  { value: "transfers", label: "Transfers" },
];

function buildActivityFeed(
  transactions: TransactionsQuery["transactions"],
  cashMovements: ActivityCashMovementsQuery["cashMovements"],
  transfers: ActivityTransfersQuery["transfers"],
): ActivityItem[] {
  const items: ActivityItem[] = [
    ...transactions.map((t) => ({
      kind: "trade" as const,
      date: t.tradeDate,
      id: `trade-${t.id}`,
      data: t,
    })),
    ...cashMovements.map((c) => ({
      kind: "cash" as const,
      date: c.date,
      id: `cash-${c.id}`,
      data: c,
    })),
    ...transfers.map((t) => ({
      kind: "transfer" as const,
      date: t.transferDate,
      id: `transfer-${t.id}`,
      data: t,
    })),
  ];

  return items.sort((a, b) => {
    if (b.date !== a.date) return b.date.localeCompare(a.date);
    return b.id.localeCompare(a.id);
  });
}

export function ActivityHistoryCard() {
  const { hideValues } = useUiState();
  const [isLocked, setIsLocked] = useState(true);
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [editingTransactionId, setEditingTransactionId] = useState<number | null>(null);
  const [activeCreateAction, setActiveCreateAction] = useState<ActivityCreateAction | null>(null);
  const [showActionPicker, setShowActionPicker] = useState(false);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);
  const [activeFilter, setActiveFilter] = useState<ActivityFilter>("all");

  const accountIdVar = selectedAccountId ? parseInt(selectedAccountId) : null;

  const { data: accountsData, loading: accountsLoading, error: accountsError, refetch: refetchAccounts } =
    useQuery<TransactionAccountsQuery>(ACCOUNTS_QUERY);
  const { data: assetsData, loading: assetsLoading, error: assetsError, refetch: refetchAssets } =
    useQuery<TransactionAssetsQuery>(ASSETS_QUERY);
  const { data: transactionsData, error: transactionsError, refetch: refetchTransactions } =
    useQuery<TransactionsQuery>(TRANSACTIONS_QUERY, { variables: { accountId: accountIdVar } });
  const { data: cashMovementsData, error: cashMovementsError, refetch: refetchCashMovements } =
    useQuery<ActivityCashMovementsQuery>(CASH_MOVEMENTS_QUERY, { variables: { accountId: accountIdVar } });
  const { data: transfersData, error: transfersError, refetch: refetchTransfers } =
    useQuery<ActivityTransfersQuery>(TRANSFERS_QUERY, { variables: { accountId: accountIdVar } });

  const [deleteTransactionMutation] = useMutation(DELETE_TRANSACTION_MUTATION, {
    refetchQueries: ["Assets", "Portfolio", "Transactions"],
  });

  const accounts = accountsData?.accounts ?? [];
  const assets = assetsData?.assets ?? [];
  const transactions = transactionsData?.transactions ?? [];
  const cashMovements = cashMovementsData?.cashMovements ?? [];
  const transfers = transfersData?.transfers ?? [];
  const assetById = new Map(assets.map((asset) => [asset.id, asset]));
  const accountById = new Map(accounts.map((account) => [account.id, account]));

  const allItems = buildActivityFeed(transactions, cashMovements, transfers);
  const filteredItems = allItems.filter((item) => {
    if (activeFilter === "all") return true;
    if (activeFilter === "trades") return item.kind === "trade";
    if (activeFilter === "cash") return item.kind === "cash";
    if (activeFilter === "transfers") return item.kind === "transfer";
    return true;
  });

  const loading = accountsLoading || assetsLoading;
  const initialDataError = accountsError ?? assetsError;
  const activityError = transactionsError ?? cashMovementsError ?? transfersError;
  const pageError = initialDataError ?? activityError;
  const pageErrorMessage = initialDataError ? "Failed to load initial data" : "Failed to load transactions";
  const selectedAccount = accounts.find((account) => String(account.id) === selectedAccountId) ?? null;
  const editingTransaction = editingTransactionId
    ? transactions.find((t) => t.id === editingTransactionId) ?? null
    : null;

  const handleDeleteClick = async (transactionId: number) => {
    if (!window.confirm("Are you sure you want to delete this transaction?")) {
      return;
    }

    setIsDeleting(transactionId);
    try {
      await deleteTransactionMutation({ variables: { id: transactionId } });
    } catch (error) {
      alert(extractGqlErrorMessage(error, "Failed to delete transaction"));
    } finally {
      setIsDeleting(null);
    }
  };

  const handleModalSaved = () => {
    setShowActionPicker(false);
    setActiveCreateAction(null);
    setEditingTransactionId(null);
  };

  const handleModalClose = () => {
    setShowActionPicker(false);
    setActiveCreateAction(null);
    setEditingTransactionId(null);
  };

  if (loading && allItems.length === 0 && accounts.length === 0) {
    return <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />;
  }

  if (pageError && allItems.length === 0) {
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
            void refetchCashMovements();
            void refetchTransfers();
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
            <h1 className="flex-1 text-2xl font-semibold tracking-tight">Activity</h1>
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
              aria-label="Add Activity"
              disabled={!selectedAccountId}
              onClick={() => {
                setEditingTransactionId(null);
                setShowActionPicker(true);
              }}
              size="icon-lg"
              title="Add Activity"
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
          <div className="flex gap-1 pt-1">
            {FILTER_LABELS.map(({ value, label }) => (
              <button
                className={cn(
                  "rounded-full px-3 py-1 text-xs font-medium transition-colors",
                  activeFilter === value
                    ? "bg-foreground text-background"
                    : "bg-muted text-muted-foreground hover:bg-muted/70",
                )}
                key={value}
                onClick={() => setActiveFilter(value)}
                type="button"
              >
                {label}
              </button>
            ))}
          </div>
        </CardHeader>
        <CardContent className="min-w-0 pt-4">
          {filteredItems.length === 0 ? (
            <ActivityHistoryCardEmptyState />
          ) : (
            <>
              <div className="space-y-2 sm:hidden">
                {filteredItems.map((item) => (
                  <ActivityHistoryCardMobileItem
                    accountById={accountById}
                    assetById={assetById}
                    hideValues={hideValues}
                    isDeleting={isDeleting}
                    isEditing={item.kind === "trade" && editingTransactionId === item.data.id}
                    isLocked={isLocked}
                    item={item}
                    key={item.id}
                    onDeleteClick={handleDeleteClick}
                    onEditClick={(t) => {
                      setEditingTransactionId(t.id);
                      setActiveCreateAction("trade");
                    }}
                  />
                ))}
              </div>

              <div className="hidden w-full overflow-x-auto sm:block">
                <table className="w-full table-fixed text-sm">
                  <thead>
                    <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                      <th className="w-[100px] pb-3 pr-4">Date</th>
                      <th className="pb-3 pr-4">Description</th>
                      <th className="w-[110px] pb-3 pr-4">Type</th>
                      <th className="w-[100px] pb-3 pr-4 text-right">Qty</th>
                      <th className="w-[100px] pb-3 pr-4 text-right">Price</th>
                      <th className="w-[130px] pb-3 pr-4 text-right">Amount</th>
                      <th className="w-[60px] pb-3 pr-4">Curr</th>
                      <th className="pb-3 pr-4">Notes</th>
                      {!isLocked ? <th className="w-[90px] pb-3 text-right">Actions</th> : null}
                    </tr>
                  </thead>
                  <tbody className="divide-y">
                    {filteredItems.map((item) => (
                      <ActivityHistoryCardDesktopRow
                        accountById={accountById}
                        assetById={assetById}
                        hideValues={hideValues}
                        isDeleting={isDeleting}
                        isEditing={item.kind === "trade" && editingTransactionId === item.data.id}
                        isLocked={isLocked}
                        item={item}
                        key={item.id}
                        onDeleteClick={handleDeleteClick}
                        onEditClick={(t) => {
                          setEditingTransactionId(t.id);
                          setActiveCreateAction("trade");
                        }}
                      />
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <ActivityActionPickerModal
        canTransfer={accounts.length >= 2}
        onClose={handleModalClose}
        onSelect={(action) => {
          setShowActionPicker(false);
          setActiveCreateAction(action);
        }}
        open={showActionPicker}
        selectedAccountName={selectedAccount?.name ?? null}
      />

      <TransactionFormModal
        key={activeCreateAction === "trade" ? (editingTransactionId ?? "new") : "closed"}
        accounts={accounts}
        assets={assets}
        editingTransaction={editingTransaction}
        onClose={handleModalClose}
        onSaved={handleModalSaved}
        open={activeCreateAction === "trade"}
        selectedAccountId={selectedAccountId}
      />

      <CashMovementFormModal
        key={activeCreateAction === "deposit" ? `deposit-${selectedAccountId}` : `closed-deposit`}
        account={selectedAccount}
        kind="deposit"
        onClose={handleModalClose}
        onSaved={handleModalSaved}
        open={activeCreateAction === "deposit"}
      />

      <CashMovementFormModal
        key={activeCreateAction === "withdraw" ? `withdraw-${selectedAccountId}` : `closed-withdraw`}
        account={selectedAccount}
        kind="withdraw"
        onClose={handleModalClose}
        onSaved={handleModalSaved}
        open={activeCreateAction === "withdraw"}
      />

      <TransferFormModal
        key={activeCreateAction === "transfer" ? `transfer-${selectedAccountId}` : "closed-transfer"}
        accounts={accounts}
        initialFromAccountId={selectedAccountId}
        onClose={handleModalClose}
        onSaved={handleModalSaved}
        open={activeCreateAction === "transfer"}
      />
    </>
  );
}
