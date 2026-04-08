import { cleanup, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";

import { UiStateProvider } from "@/lib/ui-state-provider";

import { ActivityHistoryCard } from "./ActivityHistoryCard";

function createTestClient() {
  return new ApolloClient({ link: new HttpLink({ uri: "http://localhost/graphql" }), cache: new InMemoryCache() });
}

function gqlResponse(data: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify({ data }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

function renderCard() {
  return render(
    <ApolloProvider client={createTestClient()}>
      <UiStateProvider>
        <MemoryRouter>
          <ActivityHistoryCard />
        </MemoryRouter>
      </UiStateProvider>
    </ApolloProvider>,
  );
}

describe("ActivityHistoryCard", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("trims trailing zero decimals and hides empty notes", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts: [{ id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD" }] });
      if (query.includes("assets")) return gqlResponse({ assets: [{ id: 1, symbol: "AAPL", name: "Apple Inc.", assetType: "stock" }] });
      if (query.includes("transactions")) {
        return gqlResponse({
          transactions: [{
            id: 1, accountId: 1, assetId: 1, transactionType: "BUY",
            tradeDate: "2026-03-23", quantity: "10.00", unitPrice: "150.00",
            currencyCode: "USD", notes: null,
          }],
        });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    renderCard();

    const historyCard = (await screen.findByText("Activity")).closest('[data-slot="card"]');
    const mobileList = historyCard?.querySelector(".space-y-2.sm\\:hidden");

    expect(within(mobileList as HTMLElement).getByText("10")).toBeTruthy();
    expect(within(mobileList as HTMLElement).getByText("150")).toBeTruthy();
    expect(within(mobileList as HTMLElement).getByText("1,500.00")).toBeTruthy();
    expect(within(mobileList as HTMLElement).queryByText("Notes")).toBeNull();
    expect(within(mobileList as HTMLElement).queryByText("—")).toBeNull();
  });

  it("hides actions in lock mode", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts: [] });
      if (query.includes("assets")) return gqlResponse({ assets: [] });
      if (query.includes("transactions")) {
        return gqlResponse({
          transactions: [{
            id: 1, accountId: 1, assetId: 1, transactionType: "BUY",
            tradeDate: "2026-03-23", quantity: "10.00", unitPrice: "150.00",
            currencyCode: "USD", notes: "Sample note",
          }],
        });
      }
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    renderCard();

    await screen.findByText("Activity");

    expect(screen.queryByText("Actions")).toBeNull();
    expect(screen.queryByTitle("Edit transaction")).toBeNull();
    expect(screen.queryByTitle("Delete transaction")).toBeNull();
  });

  it("shows account selector with accounts", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";
      if (query.includes("accounts")) return gqlResponse({ accounts: [{ id: 1, name: "Main Account", accountType: "BANK", baseCurrency: "USD" }] });
      if (query.includes("assets")) return gqlResponse({ assets: [] });
      if (query.includes("transactions")) return gqlResponse({ transactions: [] });
      if (query.includes("cashMovements")) return gqlResponse({ cashMovements: [] });
      if (query.includes("transfers")) return gqlResponse({ transfers: [] });
      return Promise.reject(new Error(`Unhandled: ${query}`));
    });

    renderCard();

    await screen.findByText("Activity");

    expect(screen.getByLabelText("Account:")).toBeTruthy();
    expect(screen.getAllByText("Main Account").length).toBeGreaterThan(0);
  });
});
