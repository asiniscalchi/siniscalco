import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
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

  it("shows no account selected initially", async () => {
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(JSON.stringify([]), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(
          new Response(JSON.stringify([]), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    expect(await screen.findByText("No account selected")).toBeTruthy();
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
        notes: "Initial buy",
      },
    ];

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(JSON.stringify(accounts), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(
          new Response(JSON.stringify(assets), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      if (url.includes("/transactions") && url.includes("account_id=1")) {
        return Promise.resolve(
          new Response(JSON.stringify(transactions), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    expect(await screen.findByText("AAPL")).toBeTruthy();
    expect(screen.getByText("2026-03-23")).toBeTruthy();
    expect(screen.getByText("10.00000000")).toBeTruthy();
    expect(screen.getByText("150.00")).toBeTruthy();
    expect(screen.getByText("USD")).toBeTruthy();
  });

  it("shows asset empty state and disables transaction submission when no assets exist", async () => {
    const accounts = [
      { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
    ];

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(JSON.stringify(accounts), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(
          new Response(JSON.stringify([]), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(
          new Response(JSON.stringify([]), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    const accountSelect = await screen.findByLabelText("Account:");
    fireEvent.change(accountSelect, { target: { value: "1" } });

    expect(
      await screen.findByText(
        "No assets available. Please create an asset in the Assets page first.",
      ),
    ).toBeTruthy();

    expect(
      (
        screen.getByRole("button", {
          name: "Add Transaction",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
  });

  it("edits an existing transaction", async () => {
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
      if (url.endsWith("/accounts")) {
        return Promise.resolve(new Response(JSON.stringify(accounts)));
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(new Response(JSON.stringify(assets)));
      }
      if (url.includes("/transactions/123") && init?.method === "PUT") {
        return Promise.resolve(new Response(JSON.stringify(transactions[0])));
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(new Response(JSON.stringify(transactions)));
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    // Click Edit icon button
    const editButton = await screen.findByRole("button", { name: "Edit" });
    fireEvent.click(editButton);

    // Form should change
    expect(screen.getByText("Edit Transaction")).toBeTruthy();
    expect((screen.getByLabelText("Notes") as HTMLInputElement).value).toBe("Old notes");

    // Change something
    fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "New notes" } });
    fireEvent.click(screen.getByRole("button", { name: "Save Changes" }));

    await waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledWith(
        expect.stringContaining("/transactions/123"),
        expect.objectContaining({
          method: "PUT",
          body: expect.stringContaining('"notes":"New notes"'),
        }),
      );
    });

    // Form should reset
    await waitFor(() => {
      expect(screen.queryByText("Edit Transaction")).toBeNull();
      expect(screen.getByText("Add Transaction", { selector: '[data-slot="card-title"]' })).toBeTruthy();
    });
  });

  it("deletes a transaction after confirmation", async () => {
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
        notes: "",
      },
    ];

    vi.stubGlobal("confirm", vi.fn(() => true));

    vi.mocked(fetch).mockImplementation((input, init) => {
      const url = String(input);
      if (url.endsWith("/accounts")) {
        return Promise.resolve(new Response(JSON.stringify(accounts)));
      }
      if (url.endsWith("/assets")) {
        return Promise.resolve(new Response(JSON.stringify(assets)));
      }
      if (url.includes("/transactions/123") && init?.method === "DELETE") {
        return Promise.resolve(new Response(null, { status: 204 }));
      }
      if (url.includes("/transactions")) {
        return Promise.resolve(new Response(JSON.stringify(transactions)));
      }
      return Promise.reject(new Error(`Unhandled fetch request: ${url}`));
    });

    renderTransactionsPage();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    const deleteButton = await screen.findByRole("button", { name: "Delete" });
    fireEvent.click(deleteButton);

    expect(window.confirm).toHaveBeenCalled();

    await waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledWith(
        expect.stringContaining("/transactions/123"),
        expect.objectContaining({
          method: "DELETE",
        }),
      );
    });
  });
});
