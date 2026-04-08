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

import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";

import { ASSETS_QUERY } from "@/pages/assets/assets-query";

import { UiStateProvider } from "@/lib/ui-state-provider";
import { ActivityPage } from ".";

function createTestClient() {
  return new ApolloClient({ link: new HttpLink({ uri: "http://localhost/graphql" }), cache: new InMemoryCache() });
}

function renderActivityPage() {
  return render(
    <ApolloProvider client={createTestClient()}>
      <UiStateProvider>
        <MemoryRouter>
          <ActivityPage />
        </MemoryRouter>
      </UiStateProvider>
    </ApolloProvider>,
  );
}

async function unlockEditMode() {
  const unlockButton = await screen.findByRole("button", {
    name: /unlock edit mode/i,
  });
  fireEvent.click(unlockButton);
}

function gqlResponse(data: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify({ data }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

describe("ActivityPage", () => {
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
        accountId: 1,
        assetId: 1,
        transactionType: "BUY",
        tradeDate: "2026-03-23",
        quantity: "10.00",
        unitPrice: "150.00",
        currencyCode: "USD",
        notes: "All trans",
        createdAt: "2026-03-23T00:00:00Z",
        updatedAt: "2026-03-23T00:00:00Z",
      },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) {
        return gqlResponse({ accounts: [] });
      }
      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }
      if (query.includes("transactions")) {
        return gqlResponse({ transactions });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled GQL query: ${query}`));
    });

    renderActivityPage();

    expect(await screen.findByText("Activity")).toBeTruthy();
    expect(screen.getByTitle("All trans")).toBeTruthy();
    const historyCard = screen
      .getByText("Activity")
      .closest('[data-slot="card"]');
    const mobileList = historyCard?.querySelector(".space-y-2.sm\\:hidden");

    expect(within(mobileList as HTMLElement).getByText("10")).toBeTruthy();
    expect(within(mobileList as HTMLElement).getByText("150")).toBeTruthy();
    expect(within(mobileList as HTMLElement).getByText("1,500.00")).toBeTruthy();
    expect(within(mobileList as HTMLElement).queryByText("10.00")).toBeNull();
    expect(within(mobileList as HTMLElement).queryByText("150.00")).toBeNull();
    expect((screen.getByRole("button", { name: "Add Transaction" }) as HTMLButtonElement).disabled).toBe(true);

    expect(screen.queryByText("Actions")).toBeNull();
    expect(screen.queryByTitle("Edit transaction")).toBeNull();
    expect(screen.queryByTitle("Delete transaction")).toBeNull();
  });

  it("hides empty notes in the transaction history", async () => {
    const transactions = [
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
        createdAt: "2026-03-23T00:00:00Z",
        updatedAt: "2026-03-23T00:00:00Z",
      },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) {
        return gqlResponse({ accounts: [] });
      }
      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }
      if (query.includes("transactions")) {
        return gqlResponse({ transactions });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled GQL query: ${query}`));
    });

    renderActivityPage();

    await screen.findAllByText("Date");
    const mobileList = screen
      .getByText("Activity")
      .closest('[data-slot="card"]')
      ?.querySelector(".sm\\:hidden");

    expect(within(mobileList as HTMLElement).queryByText("Notes")).toBeNull();
    expect(within(mobileList as HTMLElement).queryByText("—")).toBeNull();
  });

  it("clears a transaction load error after a successful retry", async () => {
    let transactionFetchCount = 0;

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) {
        return gqlResponse({
          accounts: [
            { id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD", summaryStatus: "OK", cashTotalAmount: null, assetTotalAmount: null, totalAmount: null, totalCurrency: null },
          ],
        });
      }
      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }
      if (query.includes("transactions")) {
        transactionFetchCount += 1;
        if (transactionFetchCount === 1) {
          return Promise.resolve(new Response(null, { status: 500 }));
        }
        return gqlResponse({ transactions: [] });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled GQL query: ${query}`));
    });

    renderActivityPage();

    expect(await screen.findByText("Failed to load transactions")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText("No transactions recorded.")).toBeTruthy();
    expect(screen.queryByText("Failed to load transactions")).toBeNull();
  });

  it("retries initial data after a load failure", async () => {
    let accountsFetchCount = 0;

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) {
        accountsFetchCount += 1;
        if (accountsFetchCount === 1) {
          return Promise.resolve(new Response(null, { status: 500 }));
        }
        return gqlResponse({
          accounts: [
            { id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD", summaryStatus: "OK", cashTotalAmount: null, assetTotalAmount: null, totalAmount: null, totalCurrency: null },
          ],
        });
      }
      if (query.includes("assets")) {
        return gqlResponse({
          assets: [
            { id: 1, symbol: "AAPL", name: "Apple Inc", assetType: "stock", quoteSymbol: null, isin: null, currentPrice: null, currentPriceCurrency: null, currentPriceAsOf: null, totalQuantity: null },
          ],
        });
      }
      if (query.includes("transactions")) {
        return gqlResponse({ transactions: [] });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled GQL query: ${query}`));
    });

    renderActivityPage();

    expect(await screen.findByText("Failed to load initial data")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText("No transactions recorded.")).toBeTruthy();
    expect(screen.queryByText("Failed to load initial data")).toBeNull();
  });

  it("loads transactions when an account is selected", async () => {
    const accounts = [
      { id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD", summaryStatus: "OK", cashTotalAmount: null, assetTotalAmount: null, totalAmount: null, totalCurrency: null },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", assetType: "stock", quoteSymbol: null, isin: null, currentPrice: null, currentPriceCurrency: null, currentPriceAsOf: null, totalQuantity: null },
    ];
    const transactions = [
      {
        id: 1,
        accountId: 1,
        assetId: 1,
        transactionType: "BUY",
        tradeDate: "2026-03-23",
        quantity: "10.00000000",
        unitPrice: "150.00000000",
        currencyCode: "USD",
        notes: "Filtered trans",
        createdAt: "2026-03-23T00:00:00Z",
        updatedAt: "2026-03-23T00:00:00Z",
      },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";
      const variables = body?.variables ?? {};
      if (query.includes("accounts")) {
        return gqlResponse({ accounts });
      }
      if (query.includes("assets")) {
        return gqlResponse({ assets });
      }
      if (query.includes("transactions")) {
        if (variables.accountId === 1) {
          return gqlResponse({ transactions });
        }
        return gqlResponse({ transactions: [] });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled GQL query: ${query}`));
    });

    renderActivityPage();

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    expect(await screen.findByTitle("Filtered trans")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Add Transaction" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("puts the account selector below the title row on mobile", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) {
        return gqlResponse({
          accounts: [
            { id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD", summaryStatus: "OK", cashTotalAmount: null, assetTotalAmount: null, totalAmount: null, totalCurrency: null },
          ],
        });
      }
      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }
      if (query.includes("transactions")) {
        return gqlResponse({ transactions: [] });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled GQL query: ${query}`));
    });

    renderActivityPage();

    await screen.findByLabelText("Account:");

    const titleRow = screen.getByText("Activity").closest("div");
    const mobileSelectRow = screen.getByLabelText("Account").closest("div");

    expect(titleRow).toBeTruthy();
    expect(mobileSelectRow?.className).toContain("justify-end");
    expect(mobileSelectRow?.className).toContain("sm:hidden");
  });

  it("keeps non-empty transaction history constrained on mobile when edit mode is unlocked", async () => {
    const accounts = [
      { id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD", summaryStatus: "OK", cashTotalAmount: null, assetTotalAmount: null, totalAmount: null, totalCurrency: null },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", assetType: "stock", quoteSymbol: null, isin: null, currentPrice: null, currentPriceCurrency: null, currentPriceAsOf: null, totalQuantity: null },
    ];
    const transactions = [
      {
        id: 1,
        accountId: 1,
        assetId: 1,
        transactionType: "BUY",
        tradeDate: "2026-03-23",
        quantity: "10.00000000",
        unitPrice: "150.00000000",
        currencyCode: "USD",
        notes: "Overflow regression",
        createdAt: "2026-03-23T00:00:00Z",
        updatedAt: "2026-03-23T00:00:00Z",
      },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) {
        return gqlResponse({ accounts });
      }
      if (query.includes("assets")) {
        return gqlResponse({ assets });
      }
      if (query.includes("transactions")) {
        return gqlResponse({ transactions });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled GQL query: ${query}`));
    });

    renderActivityPage();

    await screen.findAllByText("Overflow regression");
    await unlockEditMode();

    const historyCard = screen
      .getByText("Activity")
      .closest('[data-slot="card"]');
    const mobileList = historyCard?.querySelector(".space-y-2.sm\\:hidden");
    const desktopTable = historyCard?.querySelector(".sm\\:block");
    const mobileGrid = mobileList?.querySelector("dl");
    const mobileCardContent = mobileList?.querySelector('[data-slot="card-content"]');

    expect(historyCard).toBeTruthy();
    expect(historyCard?.className).toContain("min-w-0");
    expect(mobileList?.className).toContain("sm:hidden");
    expect(desktopTable?.className).toContain("sm:block");
    expect(mobileGrid?.className).toContain("grid-cols-4");
    expect(mobileCardContent?.className).toContain("p-3");
    expect(screen.getByText("Actions")).toBeTruthy();
  });

  it("handles create transaction via modal", async () => {
    const accounts = [
      { id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD", summaryStatus: "OK", cashTotalAmount: null, assetTotalAmount: null, totalAmount: null, totalCurrency: null },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", assetType: "stock", quoteSymbol: null, isin: null, currentPrice: null, currentPriceCurrency: null, currentPriceAsOf: null, totalQuantity: null },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts });
      if (query.includes("assets")) return gqlResponse({ assets });
      if (query.includes("createTransaction")) {
        return gqlResponse({
          createTransaction: {
            id: 1,
            accountId: 1,
            assetId: 1,
            transactionType: "BUY",
            tradeDate: "2026-03-23",
            quantity: "10",
            unitPrice: "150",
            currencyCode: "USD",
            notes: null,
            createdAt: "2026-03-23T00:00:00Z",
            updatedAt: "2026-03-23T00:00:00Z",
          },
        });
      }
      if (query.includes("transactions")) return gqlResponse({ transactions: [] });
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    renderActivityPage();

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
      { id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD", summaryStatus: "OK", cashTotalAmount: null, assetTotalAmount: null, totalAmount: null, totalCurrency: null },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", assetType: "stock", quoteSymbol: null, isin: null, currentPrice: null, currentPriceCurrency: null, currentPriceAsOf: null, totalQuantity: null },
    ];
    const transactions = [
      {
        id: 123,
        accountId: 1,
        assetId: 1,
        transactionType: "BUY",
        tradeDate: "2026-03-23",
        quantity: "10.00",
        unitPrice: "150.00",
        currencyCode: "USD",
        notes: "Old notes",
        createdAt: "2026-03-23T00:00:00Z",
        updatedAt: "2026-03-23T00:00:00Z",
      },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts });
      if (query.includes("assets")) return gqlResponse({ assets });
      if (query.includes("updateTransaction")) {
        return gqlResponse({ updateTransaction: transactions[0] });
      }
      if (query.includes("transactions")) return gqlResponse({ transactions });
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    renderActivityPage();

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
        accountId: 1,
        assetId: 1,
        transactionType: "BUY",
        tradeDate: "2026-03-23",
        quantity: "10.00",
        unitPrice: "150.00",
        currencyCode: "USD",
        notes: "To be deleted",
        createdAt: "2026-03-23T00:00:00Z",
        updatedAt: "2026-03-23T00:00:00Z",
      },
    ];

    vi.stubGlobal("confirm", vi.fn(() => true));

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts: [] });
      if (query.includes("assets")) return gqlResponse({ assets: [] });
      if (query.includes("deleteTransaction")) {
        return gqlResponse({ deleteTransaction: 1 });
      }
      if (query.includes("transactions")) {
        return gqlResponse({ transactions });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    renderActivityPage();
    await unlockEditMode();

    fireEvent.click(screen.getAllByTitle("Delete transaction")[0]);

    expect(window.confirm).toHaveBeenCalled();

    await waitFor(() => {
      const calls = vi.mocked(fetch).mock.calls;
      const hasDeleteCall = calls.some(
        ([, opts]) =>
          opts?.body != null && String(opts.body).includes("deleteTransaction"),
      );
      expect(hasDeleteCall).toBe(true);
    });
  });

  it("uses the transaction's own accountId when editing in All Accounts view", async () => {
    const accounts = [
      { id: 42, name: "Broker Account", accountType: "BROKER", baseCurrency: "USD" },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", assetType: "STOCK" },
    ];
    const transactions = [
      {
        id: 123,
        accountId: 42,
        assetId: 1,
        transactionType: "BUY",
        tradeDate: "2026-01-01",
        quantity: "10.00",
        unitPrice: "150.00",
        currencyCode: "USD",
        notes: "test",
      },
    ];

    let capturedAccountId: number | null | undefined = undefined;

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts });
      if (query.includes("updateTransaction")) {
        const input = (body?.variables as { input?: { accountId?: number } } | undefined)?.input;
        capturedAccountId = input?.accountId;
        return gqlResponse({ updateTransaction: transactions[0] });
      }
      if (query.includes("assets")) return gqlResponse({ assets });
      if (query.includes("transactions")) return gqlResponse({ transactions });
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    renderActivityPage();

    // Stay in "All Accounts" view (do NOT select an account)
    await screen.findByLabelText("Account:");
    await screen.findByTitle("test");

    await unlockEditMode();
    fireEvent.click(screen.getAllByTitle("Edit transaction")[0]);

    const modal = screen.getByRole("dialog");
    fireEvent.change(within(modal).getByLabelText("Notes"), { target: { value: "updated" } });
    fireEvent.click(within(modal).getByRole("button", { name: "Save Changes" }));

    await waitFor(() => {
      expect(capturedAccountId).toBe(42);
    });
  });

  it("evicts assets and portfolio cache after updating a transaction", async () => {
    const client = new ApolloClient({
      link: new HttpLink({ uri: "http://localhost/graphql" }),
      cache: new InMemoryCache(),
    });

    // Simulate a prior visit to the assets page by writing to the Apollo cache
    client.writeQuery({
      query: ASSETS_QUERY,
      data: {
        assets: [
          {
            __typename: "Asset",
            id: 1,
            symbol: "AAPL",
            name: "Apple Inc",
            assetType: "STOCK",
            quoteSymbol: null,
            isin: null,
            currentPrice: null,
            currentPriceCurrency: null,
            currentPriceAsOf: null,
            totalQuantity: "10.00000000",
            avgCostBasis: null,
            avgCostBasisCurrency: null,
            previousClose: null,
            previousCloseCurrency: null,
            convertedTotalValue: null,
            convertedTotalValueCurrency: null,
          },
        ],
      },
    });

    expect(client.readQuery({ query: ASSETS_QUERY })).not.toBeNull();

    const accounts = [
      { id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD" },
    ];
    const assets = [
      { id: 1, symbol: "AAPL", name: "Apple Inc", assetType: "STOCK" },
    ];
    const transactions = [
      {
        id: 123,
        accountId: 1,
        assetId: 1,
        transactionType: "BUY",
        tradeDate: "2026-01-01",
        quantity: "10.00",
        unitPrice: "150.00",
        currencyCode: "USD",
        notes: "old notes",
      },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts });
      if (query.includes("updateTransaction")) return gqlResponse({ updateTransaction: transactions[0] });
      if (query.includes("assets")) return gqlResponse({ assets });
      if (query.includes("transactions")) return gqlResponse({ transactions });
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    render(
      <ApolloProvider client={client}>
        <UiStateProvider>
          <MemoryRouter>
            <ActivityPage />
          </MemoryRouter>
        </UiStateProvider>
      </ApolloProvider>,
    );

    const select = await screen.findByLabelText("Account:");
    fireEvent.change(select, { target: { value: "1" } });

    await unlockEditMode();
    fireEvent.click(screen.getAllByTitle("Edit transaction")[0]);

    const modal = screen.getByRole("dialog");
    fireEvent.change(within(modal).getByLabelText("Notes"), { target: { value: "new notes" } });
    fireEvent.click(within(modal).getByRole("button", { name: "Save Changes" }));

    await waitFor(() => {
      expect(client.readQuery({ query: ASSETS_QUERY })).toBeNull();
    });
  });

  it("evicts assets and portfolio cache after deleting a transaction", async () => {
    const client = new ApolloClient({
      link: new HttpLink({ uri: "http://localhost/graphql" }),
      cache: new InMemoryCache(),
    });

    // Simulate a prior visit to the assets page by writing to the Apollo cache
    client.writeQuery({
      query: ASSETS_QUERY,
      data: {
        assets: [
          {
            __typename: "Asset",
            id: 1,
            symbol: "AAPL",
            name: "Apple Inc",
            assetType: "STOCK",
            quoteSymbol: null,
            isin: null,
            currentPrice: null,
            currentPriceCurrency: null,
            currentPriceAsOf: null,
            totalQuantity: "10.00000000",
            avgCostBasis: null,
            avgCostBasisCurrency: null,
            previousClose: null,
            previousCloseCurrency: null,
            convertedTotalValue: null,
            convertedTotalValueCurrency: null,
          },
        ],
      },
    });

    expect(client.readQuery({ query: ASSETS_QUERY })).not.toBeNull();

    vi.stubGlobal("confirm", vi.fn(() => true));

    const transactions = [
      {
        id: 123,
        accountId: 1,
        assetId: 1,
        transactionType: "BUY",
        tradeDate: "2026-01-01",
        quantity: "10.00",
        unitPrice: "150.00",
        currencyCode: "USD",
        notes: "to be deleted",
      },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts: [] });
      if (query.includes("deleteTransaction")) return gqlResponse({ deleteTransaction: 123 });
      if (query.includes("assets")) return gqlResponse({ assets: [] });
      if (query.includes("transactions")) return gqlResponse({ transactions });
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    render(
      <ApolloProvider client={client}>
        <UiStateProvider>
          <MemoryRouter>
            <ActivityPage />
          </MemoryRouter>
        </UiStateProvider>
      </ApolloProvider>,
    );

    await unlockEditMode();
    fireEvent.click(screen.getAllByTitle("Delete transaction")[0]);

    await waitFor(() => {
      expect(client.readQuery({ query: ASSETS_QUERY })).toBeNull();
    });
  });
});
