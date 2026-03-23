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
      if (url.includes("/asset-transactions") && url.includes("account_id=1")) {
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

  it("submits a new transaction successfully", async () => {
    const accounts = [
      { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", asset_type: "stock" },
    ];

    vi.mocked(fetch).mockImplementation((input, init) => {
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
      if (url.includes("/asset-transactions") && init?.method === "POST") {
        return Promise.resolve(
          new Response(JSON.stringify({ id: 2 }), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      if (url.includes("/asset-transactions")) {
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

    // Fill the form
    fireEvent.change(screen.getByLabelText("Asset"), { target: { value: "1" } });
    fireEvent.change(screen.getByLabelText("Quantity"), {
      target: { value: "5" },
    });
    fireEvent.change(screen.getByLabelText("Unit Price"), {
      target: { value: "160" },
    });

    fireEvent.click(screen.getByRole("button", { name: /Add Transaction/i }));

    await waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledWith(
        expect.stringContaining("/asset-transactions"),
        expect.objectContaining({
          method: "POST",
          body: expect.stringContaining('"quantity":"5"'),
        }),
      );
    });
  });
});
