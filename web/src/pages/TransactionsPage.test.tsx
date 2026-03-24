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
import { TransactionsPage } from "./TransactionsPage";

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
    await unlockEditMode();

    expect(await screen.findByText("Showing all recorded transactions.")).toBeTruthy();
    expect(screen.getByText("All trans")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Add Transaction" }) as HTMLButtonElement).disabled).toBe(true);
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
    await unlockEditMode();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    expect(await screen.findByText("Recent transactions for Main Account.")).toBeTruthy();
    expect(screen.getByText("Filtered trans")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Add Transaction" }) as HTMLButtonElement).disabled).toBe(false);
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
    await unlockEditMode();

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
    await unlockEditMode();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    fireEvent.click(await screen.findByTitle("Edit transaction"));

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

    fireEvent.click(await screen.findByTitle("Delete transaction"));

    expect(window.confirm).toHaveBeenCalled();

    await waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledWith(
        expect.stringContaining("/transactions/123"),
        expect.objectContaining({ method: "DELETE" }),
      );
    });
  });
});
