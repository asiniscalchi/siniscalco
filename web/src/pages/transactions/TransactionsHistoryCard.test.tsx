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
  const accounts: Account[] = [{ id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" }];
  const assets: Asset[] = [{ id: 1, symbol: "AAPL", name: "Apple Inc.", asset_type: "stock" }];

  return render(
    <TransactionsHistoryCard
      accounts={accounts}
      assets={assets}
      editingTransactionId={null}
      hideValues={false}
      isDeleting={null}
      isLocked={isLocked}
      onDeleteClick={() => undefined}
      onEditClick={() => undefined}
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
          account_id: 1,
          asset_id: 1,
          transaction_type: "BUY",
          trade_date: "2026-03-23",
          quantity: "10.00",
          unit_price: "150.00",
          currency_code: "USD",
          notes: null,
        },
      ],
    });

    const historyCard = screen.getByText("Transaction History").closest('[data-slot="card"]');
    const mobileList = historyCard?.querySelector(".sm\\:hidden");

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
          account_id: 1,
          asset_id: 1,
          transaction_type: "BUY",
          trade_date: "2026-03-23",
          quantity: "10.00",
          unit_price: "150.00",
          currency_code: "USD",
          notes: "Sample note",
        },
      ],
    });

    expect(screen.queryByText("Actions")).toBeNull();
    expect(screen.queryByTitle("Edit transaction")).toBeNull();
    expect(screen.queryByTitle("Delete transaction")).toBeNull();
  });

  it("describes the selected account when filtering", () => {
    renderHistoryCard({
      isLocked: true,
      selectedAccountId: "1",
      transactions: [],
    });

    expect(
      screen.getByText("Recent transactions for Main Account."),
    ).toBeTruthy();
  });
});
