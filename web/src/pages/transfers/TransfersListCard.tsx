import { useState } from "react";
import { gql } from "@apollo/client/core";
import { useMutation, useQuery } from "@apollo/client/react";

import { LockIcon, PlusIcon, UnlockIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { extractGqlErrorMessage } from "@/lib/gql";
import { cn } from "@/lib/utils";
import type {
  TransferAccountsQuery,
  TransfersQuery,
} from "@/gql/types";

import { TransferFormModal } from "./TransferFormModal";

const ACCOUNTS_QUERY = gql`
  query TransferAccounts {
    accounts {
      id name baseCurrency
    }
  }
`;

const TRANSFERS_QUERY = gql`
  query Transfers {
    transfers {
      id fromAccountId toAccountId
      fromCurrency fromAmount toCurrency toAmount
      transferDate notes
    }
  }
`;

const DELETE_TRANSFER_MUTATION = gql`
  mutation DeleteTransfer($id: Int!) {
    deleteTransfer(id: $id)
  }
`;

export function TransfersListCard() {
  const [isLocked, setIsLocked] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  const { data: accountsData, loading: accountsLoading, error: accountsError, refetch: refetchAccounts } =
    useQuery<TransferAccountsQuery>(ACCOUNTS_QUERY);
  const { data: transfersData, error: transfersError, refetch: refetchTransfers } =
    useQuery<TransfersQuery>(TRANSFERS_QUERY);

  const [deleteTransferMutation] = useMutation(DELETE_TRANSFER_MUTATION);

  const accounts = accountsData?.accounts ?? [];
  const transfers = transfersData?.transfers ?? [];

  const accountById = new Map(accounts.map((a) => [a.id, a]));

  const loading = accountsLoading;
  const pageError = accountsError ?? transfersError;

  const handleDelete = async (transferId: number) => {
    if (!window.confirm("Are you sure you want to delete this transfer?")) return;
    setIsDeleting(transferId);
    try {
      await deleteTransferMutation({ variables: { id: transferId } });
      await refetchTransfers();
    } catch (error) {
      alert(extractGqlErrorMessage(error, "Failed to delete transfer"));
    } finally {
      setIsDeleting(null);
    }
  };

  const handleModalSaved = () => {
    setShowModal(false);
    void refetchTransfers();
    void refetchAccounts();
  };

  if (loading && transfers.length === 0 && accounts.length === 0) {
    return <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />;
  }

  if (pageError && transfers.length === 0) {
    return (
      <Card className="border-destructive/30 bg-background">
        <CardHeader>
          <CardTitle>Error</CardTitle>
          <CardDescription>Failed to load transfers</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={() => { void refetchAccounts(); void refetchTransfers(); }}>
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
            <h1 className="flex-1 text-2xl font-semibold tracking-tight">Transfers</h1>
            <Button
              aria-label={isLocked ? "Unlock edit mode" : "Lock edit mode"}
              className={cn(
                "size-9 rounded-full transition-colors",
                !isLocked &&
                  "bg-amber-100 text-amber-900 hover:bg-amber-200 dark:bg-amber-900/30 dark:text-amber-400 dark:hover:bg-amber-900/50",
              )}
              onClick={() => setIsLocked((l) => !l)}
              size="icon"
              title={isLocked ? "Unlock edit mode" : "Lock edit mode"}
              type="button"
              variant="ghost"
            >
              {isLocked ? <LockIcon /> : <UnlockIcon />}
            </Button>
            <Button
              aria-label="New Transfer"
              disabled={accounts.length < 2}
              onClick={() => setShowModal(true)}
              size="icon-lg"
              title="New Transfer"
            >
              <PlusIcon />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="min-w-0 pt-4">
          {transfers.length === 0 ? (
            <p className="py-12 text-center text-sm text-muted-foreground">
              No transfers yet. Use the + button to record a transfer.
            </p>
          ) : (
            <div className="w-full overflow-x-auto">
              <table className="w-full table-fixed text-sm">
                <thead>
                  <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                    <th className="w-[100px] pb-3 pr-4">Date</th>
                    <th className="pb-3 pr-4">From</th>
                    <th className="pb-3 pr-4">To</th>
                    <th className="w-[130px] pb-3 pr-4 text-right">Amount Sent</th>
                    <th className="w-[130px] pb-3 pr-4 text-right">Amount Received</th>
                    <th className="pb-3 pr-4">Notes</th>
                    {!isLocked && <th className="w-[80px] pb-3 text-right">Actions</th>}
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {transfers.map((transfer) => {
                    const fromAccount = accountById.get(transfer.fromAccountId);
                    const toAccount = accountById.get(transfer.toAccountId);
                    return (
                      <tr key={transfer.id} className="group">
                        <td className="py-3 pr-4 text-muted-foreground">
                          {transfer.transferDate}
                        </td>
                        <td className="truncate py-3 pr-4">
                          {fromAccount?.name ?? `#${transfer.fromAccountId}`}
                        </td>
                        <td className="truncate py-3 pr-4">
                          {toAccount?.name ?? `#${transfer.toAccountId}`}
                        </td>
                        <td className="py-3 pr-4 text-right font-mono">
                          {transfer.fromAmount} {transfer.fromCurrency}
                        </td>
                        <td className="py-3 pr-4 text-right font-mono">
                          {transfer.toAmount} {transfer.toCurrency}
                        </td>
                        <td className="truncate py-3 pr-4 text-muted-foreground">
                          {transfer.notes ?? "—"}
                        </td>
                        {!isLocked && (
                          <td className="py-3 text-right">
                            <Button
                              className="text-destructive hover:text-destructive"
                              disabled={isDeleting === transfer.id}
                              onClick={() => handleDelete(transfer.id)}
                              size="sm"
                              type="button"
                              variant="ghost"
                            >
                              {isDeleting === transfer.id ? "Deleting…" : "Delete"}
                            </Button>
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

      <TransferFormModal
        key={showModal ? "open" : "closed"}
        accounts={accounts}
        onClose={() => setShowModal(false)}
        onSaved={handleModalSaved}
        open={showModal}
      />
    </>
  );
}
