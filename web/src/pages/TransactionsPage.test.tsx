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
      await screen.findByText("No assets yet. Create one to record a transaction."),
    ).toBeTruthy();
    
    // There are two "Create Asset" buttons: one in header, one in empty state
    const createAssetButtons = screen.getAllByRole("button", { name: "Create Asset" });
    expect(createAssetButtons.length).toBe(2);
    
    expect(
      (
        screen.getByRole("button", { name: "Add Transaction" }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
  });

  it("creates an asset via modal, refreshes assets, and auto-selects it", async () => {
    const accounts = [
      { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
    ];
    const initialAssets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", asset_type: "stock" },
    ];
    const refreshedAssets = [
      ...initialAssets,
      { id: 2, symbol: "VWCE", name: "Vanguard FTSE All-World UCITS ETF", asset_type: "ETF" },
    ];

    let assetsRequestCount = 0;

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
        if (init?.method === "POST") {
          return Promise.resolve(
            new Response(
              JSON.stringify({
                id: 2,
                symbol: "VWCE",
                name: "Vanguard FTSE All-World UCITS ETF",
                asset_type: "ETF",
                isin: null,
                created_at: "2026-03-23T12:00:00Z",
                updated_at: "2026-03-23T12:00:00Z",
              }),
              {
                status: 201,
                headers: { "Content-Type": "application/json" },
              },
            ),
          );
        }

        assetsRequestCount += 1;
        return Promise.resolve(
          new Response(JSON.stringify(assetsRequestCount >= 2 ? refreshedAssets : initialAssets), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }
      if (url.includes("/transactions") && init?.method === "POST") {
        return Promise.resolve(
          new Response(JSON.stringify({ id: 2 }), {
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

    // Open modal using the first "Create Asset" button (in header)
    fireEvent.click(screen.getAllByRole("button", { name: "Create Asset" })[0]);
    
    // Check for modal content
    expect(await screen.findByRole("heading", { name: "Create Asset" })).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Symbol *"), { target: { value: "vwce" } });
    fireEvent.change(screen.getByLabelText("Name *"), {
      target: { value: "Vanguard FTSE All-World UCITS ETF" },
    });
    fireEvent.change(screen.getByLabelText("Asset Type *"), {
      target: { value: "ETF" },
    });
    
    // Find the submit button in the modal
    const createAssetButtons = screen.getAllByRole("button", { name: "Create Asset" });
    const submitButton = createAssetButtons.find(btn => (btn as HTMLButtonElement).type === 'submit');
    if (!submitButton) throw new Error("Submit button not found");
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledWith(
        expect.stringContaining("/assets"),
        expect.objectContaining({
          method: "POST",
          body: expect.stringContaining('"symbol":"vwce"'),
        }),
      );
    });

    await waitFor(() => {
      expect((screen.getByLabelText("Asset") as HTMLSelectElement).value).toBe(
        "2",
      );
    });

    fireEvent.change(screen.getByLabelText("Quantity"), {
      target: { value: "5" },
    });
    fireEvent.change(screen.getByLabelText("Unit Price"), {
      target: { value: "160" },
    });

    fireEvent.click(screen.getByRole("button", { name: /Add Transaction/i }));

    await waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledWith(
        expect.stringContaining("/transactions"),
        expect.objectContaining({
          method: "POST",
          body: expect.stringContaining('"quantity":"5"'),
        }),
      );
    });
  });

  it("shows validation errors and preserves create-asset values in modal", async () => {
    const accounts = [
      { id: 1, name: "Main Account", account_type: "bank", base_currency: "USD" },
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
        if (init?.method === "POST") {
          return Promise.resolve(
            new Response(
              JSON.stringify({
                message: "Asset validation failed",
                field_errors: {
                  symbol: ["Symbol is required"],
                  asset_type: ["Asset type must be one of: STOCK, ETF, BOND, CRYPTO, CASH_EQUIVALENT, OTHER"],
                },
              }),
              {
                status: 422,
                headers: { "Content-Type": "application/json" },
              },
            ),
          );
        }

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

    fireEvent.click(screen.getAllByRole("button", { name: "Create Asset" })[0]);
    fireEvent.change(screen.getByLabelText("Symbol *"), { target: { value: "   " } });
    fireEvent.change(screen.getByLabelText("Name *"), { target: { value: "Pending Asset" } });
    fireEvent.change(screen.getByLabelText("Asset Type *"), {
      target: { value: "OTHER" },
    });
    
    const createAssetButtons = screen.getAllByRole("button", { name: "Create Asset" });
    const submitButton = createAssetButtons.find(btn => (btn as HTMLButtonElement).type === 'submit');
    if (!submitButton) throw new Error("Submit button not found");
    fireEvent.click(submitButton);

    expect(await screen.findByText("Asset validation failed")).toBeTruthy();
    expect(screen.getByText("Symbol is required")).toBeTruthy();
    expect((screen.getByLabelText("Name *") as HTMLInputElement).value).toBe(
      "Pending Asset",
    );
    expect((screen.getByLabelText("Asset Type *") as HTMLSelectElement).value).toBe(
      "OTHER",
    );
  });
});
