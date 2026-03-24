import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UiStateProvider } from "@/lib/ui-state-provider";
import { TransactionsPage } from ".";

function renderTransactionsPage() {
  return render(
    <UiStateProvider>
      <MemoryRouter>
        <TransactionsPage />
      </MemoryRouter>
    </UiStateProvider>,
  );
}

async function unlockEditMode() {
  const unlockButton = await screen.findByRole("button", {
    name: /unlock edit mode/i,
  });
  fireEvent.click(unlockButton);
}

describe("TransactionsPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows all transactions initially", async () => {
    const transactions = [
      {
        id: 1,
        account_id: 1,
        asset_id: 1,
        transaction_type: "BUY",
        trade_date: "2026-03-23",
        quantity: "10.00",
        unit_price: "150.00",
        currency_code: "USD",
        notes: "All trans",
      },
    ];

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(new Response(JSON.stringify(transactions)));
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    expect(await screen.findByText("Showing all recorded transactions.")).toBeTruthy();
    expect(screen.getByTitle("All trans")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Add Transaction" }) as HTMLButtonElement).disabled).toBe(true);

    expect(screen.getByText("Actions")).toBeTruthy();

    const editButton = screen.getAllByTitle("Edit transaction")[0] as HTMLButtonElement;
    const deleteButton = screen.getAllByTitle("Delete transaction")[0] as HTMLButtonElement;

    expect(editButton.disabled).toBe(true);
    expect(deleteButton.disabled).toBe(true);
  });

  it("clears a transaction load error after a successful retry", async () => {
    let transactionFetchCount = 0;

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
            ]),
          ),
        );
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      if (url.includes("/transactions")) {
        transactionFetchCount += 1;
        if (transactionFetchCount === 1) {
          return Promise.resolve(new Response(null, { status: 500 }));
        }
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    expect(await screen.findByText("Failed to load transactions")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText("No transactions recorded.")).toBeTruthy();
    expect(screen.queryByText("Failed to load transactions")).toBeNull();
  });

  it("retries initial data after a load failure", async () => {
    let accountsFetchCount = 0;

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        accountsFetchCount += 1;
        if (accountsFetchCount === 1) {
          return Promise.resolve(new Response(null, { status: 500 }));
        }
        return Promise.resolve(
          new Response(
            JSON.stringify([
              { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
            ]),
          ),
        );
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              { id: 1, symbol: "AAPL", name: "Apple Inc", asset_type: "stock" },
            ]),
          ),
        );
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    expect(await screen.findByText("Failed to load initial data")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText("Showing all recorded transactions.")).toBeTruthy();
    expect(screen.queryByText("Failed to load initial data")).toBeNull();
  });

  it("loads transactions when an account is selected", async () => {
    const accounts = [
      { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", asset_type: "stock" },
    ];
    const transactions = [
      {
        id: 1,
        account_id: 1,
        asset_id: 1,
        transaction_type: "BUY",
        trade_date: "2026-03-23",
        quantity: "10.00000000",
        unit_price: "150.00000000",
        currency_code: "USD",
        notes: "Filtered trans",
      },
    ];

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(new Response(JSON.stringify(accounts)));
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(new Response(JSON.stringify(assets)));
      }
      if (url.includes("/transactions?account_id=1")) {
        return Promise.resolve(new Response(JSON.stringify(transactions)));
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    expect(await screen.findByText("Recent transactions for Main Account.")).toBeTruthy();
    expect(screen.getByTitle("Filtered trans")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Add Transaction" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("keeps the header controls wrappable on mobile when edit mode is unlocked", async () => {
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
            ]),
          ),
        );
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    await screen.findByLabelText("Account:");
    await unlockEditMode();

    const unlockButton = screen.getByRole("button", { name: /lock edit mode/i });
    const controlsRow = unlockButton.closest("div")?.parentElement;

    expect(controlsRow).toBeTruthy();
    expect(controlsRow?.className).toContain("w-full");
    expect(controlsRow?.className).toContain("flex-wrap");
  });

  it("keeps non-empty transaction history constrained on mobile when edit mode is unlocked", async () => {
    const accounts = [
      { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", asset_type: "stock" },
    ];
    const transactions = [
      {
        id: 1,
        account_id: 1,
        asset_id: 1,
        transaction_type: "BUY",
        trade_date: "2026-03-23",
        quantity: "10.00000000",
        unit_price: "150.00000000",
        currency_code: "USD",
        notes: "Overflow regression",
      },
    ];

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(new Response(JSON.stringify(accounts)));
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(new Response(JSON.stringify(assets)));
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(new Response(JSON.stringify(transactions)));
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    await screen.findAllByText("Overflow regression");
    await unlockEditMode();

    const historyCard = screen
      .getByText("Transaction History")
      .closest('[data-slot="card"]');
    const mobileList = historyCard?.querySelector(".sm\\:hidden");
    const desktopTable = historyCard?.querySelector(".sm\\:block");

    expect(historyCard).toBeTruthy();
    expect(historyCard?.className).toContain("min-w-0");
    expect(mobileList?.className).toContain("sm:hidden");
    expect(desktopTable?.className).toContain("sm:block");
    expect(screen.getByText("Actions")).toBeTruthy();
  });

  it("handles create transaction via modal", async () => {
    const accounts = [
      { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", asset_type: "stock" },
    ];

    vi.mocked(fetch).mockImplementation((input, init) => {
      const url = String(input);
      if (url.endsWith("/accounts")) return Promise.resolve(new Response(JSON.stringify(accounts)));
      if (url.endsWith("/assets")) return Promise.resolve(new Response(JSON.stringify(assets)));
      if (url.includes("/transactions") && init?.method === "POST") {
        return Promise.resolve(new Response(JSON.stringify({ id: 1 }), { status: 201 }));
      }
      if (url.includes("/transactions")) return Promise.resolve(new Response(JSON.stringify([])));
      return Promise.reject(new Error(`Unhandled: ${url}`));
    });

    renderTransactionsPage();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    fireEvent.click(screen.getByRole("button", { name: "Add Transaction" }));

    const modal = screen.getByRole("dialog");
    expect(within(modal).getByText("Add Transaction", { selector: "h2" })).toBeTruthy();

    fireEvent.change(within(modal).getByLabelText(/Asset \*/), { target: { value: "1" } });
    fireEvent.change(within(modal).getByLabelText(/Quantity \*/), { target: { value: "10" } });
    fireEvent.change(within(modal).getByLabelText(/Unit Price \*/), { target: { value: "150" } });
    fireEvent.change(within(modal).getByLabelText(/Currency \*/), { target: { value: "USD" } });

    fireEvent.click(within(modal).getByRole("button", { name: "Add Transaction" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  it("handles edit transaction", async () => {
    const accounts = [
      { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", asset_type: "stock" },
    ];
    const transactions = [
      {
        id: 123,
        account_id: 1,
        asset_id: 1,
        transaction_type: "BUY",
        trade_date: "2026-03-23",
        quantity: "10.00",
        unit_price: "150.00",
        currency_code: "USD",
        notes: "Old notes",
      },
    ];

    vi.mocked(fetch).mockImplementation((input, init) => {
      const url = String(input);
      if (url.endsWith("/accounts")) return Promise.resolve(new Response(JSON.stringify(accounts)));
      if (url.endsWith("/assets")) return Promise.resolve(new Response(JSON.stringify(assets)));
      if (url.includes("/transactions/123") && init?.method === "PUT") {
        return Promise.resolve(new Response(JSON.stringify(transactions[0])));
      }
      if (url.includes("/transactions")) return Promise.resolve(new Response(JSON.stringify(transactions)));
      return Promise.reject(new Error(`Unhandled: ${url}`));
    });

    renderTransactionsPage();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    await unlockEditMode();
    fireEvent.click(screen.getAllByTitle("Edit transaction")[0]);

    const modal = screen.getByRole("dialog");
    expect(within(modal).getByText("Edit Transaction")).toBeTruthy();
    expect((within(modal).getByLabelText("Notes") as HTMLInputElement).value).toBe("Old notes");

    fireEvent.change(within(modal).getByLabelText("Notes"), { target: { value: "New notes" } });
    fireEvent.click(within(modal).getByRole("button", { name: "Save Changes" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  it("handles delete transaction", async () => {
    const transactions = [
      {
        id: 123,
        account_id: 1,
        asset_id: 1,
        transaction_type: "BUY",
        trade_date: "2026-03-23",
        quantity: "10.00",
        unit_price: "150.00",
        currency_code: "USD",
        notes: "To be deleted",
      },
    ];

    vi.stubGlobal("confirm", vi.fn(() => true));

    vi.mocked(fetch).mockImplementation((input, init) => {
      const url = String(input);
      if (url.endsWith("/accounts")) return Promise.resolve(new Response(JSON.stringify([])));
      if (url.endsWith("/assets")) return Promise.resolve(new Response(JSON.stringify([])));
      if (url.includes("/transactions/123") && init?.method === "DELETE") {
        return Promise.resolve(new Response(null, { status: 204 }));
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(new Response(JSON.stringify(transactions)));
      }
      return Promise.reject(new Error(`Unhandled: ${url}`));
    });

    renderTransactionsPage();
    await unlockEditMode();

    fireEvent.click(screen.getAllByTitle("Delete transaction")[0]);

    expect(window.confirm).toHaveBeenCalled();

    await waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledWith(
        expect.stringContaining("/transactions/123"),
        expect.objectContaining({ method: "DELETE" }),
      );
    });
  });
});
