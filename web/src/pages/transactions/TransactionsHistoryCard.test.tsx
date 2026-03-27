import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { TransactionsHistoryCard } from "./TransactionsHistoryCard";
import type { Account, Asset, Transaction } from "./types";

function renderHistoryCard({
  isLocked,
  selectedAccountId = "",
  transactions,
}: {
  isLocked: boolean;
  selectedAccountId?: string;
  transactions: Transaction[];
}) {
  const accounts: Account[] = [{ id: 1, name: "Main Account", accountType: "bank", baseCurrency: "USD" }];
  const assets: Asset[] = [{ id: 1, symbol: "AAPL", name: "Apple Inc.", assetType: "stock" }];

  return render(
    <TransactionsHistoryCard
      accounts={accounts}
      assets={assets}
      editingTransactionId={null}
      hideValues={false}
      isDeleting={null}
      isLocked={isLocked}
      onAccountChange={() => undefined}
      onCreateClick={() => undefined}
      onDeleteClick={() => undefined}
      onEditClick={() => undefined}
      onToggleLock={() => undefined}
      selectedAccountId={selectedAccountId}
      transactions={transactions}
    />,
  );
}

describe("TransactionsHistoryCard", () => {
  afterEach(() => {
    cleanup();
  });

  it("trims trailing zero decimals and hides empty notes", () => {
    renderHistoryCard({
      isLocked: true,
      transactions: [
        {
          id: 1,
          accountId: 1,
          assetId: 1,
          transactionType: "BUY",
          tradeDate: "2026-03-23",
          quantity: "10.00",
          unitPrice: "150.00",
          currencyCode: "USD",
          notes: null,
        },
      ],
    });

    const historyCard = screen.getByText("Transactions").closest('[data-slot="card"]');
    const mobileList = historyCard?.querySelector(".space-y-2.sm\\:hidden");

    expect(within(mobileList as HTMLElement).getByText("10")).toBeTruthy();
    expect(within(mobileList as HTMLElement).getByText("150")).toBeTruthy();
    expect(within(mobileList as HTMLElement).queryByText("Notes")).toBeNull();
    expect(within(mobileList as HTMLElement).queryByText("—")).toBeNull();
  });

  it("hides actions in lock mode", () => {
    renderHistoryCard({
      isLocked: true,
      transactions: [
        {
          id: 1,
          accountId: 1,
          assetId: 1,
          transactionType: "BUY",
          tradeDate: "2026-03-23",
          quantity: "10.00",
          unitPrice: "150.00",
          currencyCode: "USD",
          notes: "Sample note",
        },
      ],
    });

    expect(screen.queryByText("Actions")).toBeNull();
    expect(screen.queryByTitle("Edit transaction")).toBeNull();
    expect(screen.queryByTitle("Delete transaction")).toBeNull();
  });

  it("shows account selector with accounts", () => {
    renderHistoryCard({
      isLocked: true,
      selectedAccountId: "1",
      transactions: [],
    });

    expect(screen.getByLabelText("Account:")).toBeTruthy();
    expect(screen.getAllByText("Main Account").length).toBeGreaterThan(0);
  });
});
