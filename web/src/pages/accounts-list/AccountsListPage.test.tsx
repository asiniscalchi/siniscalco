import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";

import { type PortfolioSummary } from "@/lib/types";
import { UiStateProvider } from "@/lib/ui-state-provider";
import { AccountsListPage } from ".";

function createTestClient() {
  return new ApolloClient({ link: new HttpLink({ uri: "http://localhost/graphql" }), cache: new InMemoryCache() });
}

function renderAccountsListPage() {
  return render(
    <ApolloProvider client={createTestClient()}>
      <UiStateProvider>
        <MemoryRouter>
          <AccountsListPage />
        </MemoryRouter>
      </UiStateProvider>
    </ApolloProvider>,
  );
}

function gqlResponse(data: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify({ data }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

const defaultPortfolio: PortfolioSummary = {
  displayCurrency: "EUR",
  totalValueStatus: "OK",
  totalValueAmount: "100.00000000",
  dailyGainAmount: null,
  totalGainAmount: null,
  accountTotals: [],
  cashByCurrency: [],
  fxLastUpdated: null,
  fxRefreshStatus: "AVAILABLE",
  fxRefreshError: null,
  allocationTotals: [],
  allocationIsPartial: false,
  holdings: [],
  holdingsIsPartial: false,
};

function mockDashboardRequests({
  accounts,
  portfolio = defaultPortfolio,
}: {
  accounts: unknown[];
  portfolio?: typeof defaultPortfolio;
}) {
  vi.mocked(fetch).mockImplementation((_input, init) => {
    const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
    const query = body?.query ?? "";

    if (query.includes("accounts {")) {
      return gqlResponse({ accounts });
    }

    if (query.includes("portfolio {")) {
      return gqlResponse({ portfolio });
    }

    throw new Error(`Unhandled GQL query: ${query}`);
  });
}

describe("AccountsListPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows a loading state before accounts resolve", () => {
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}));

    renderAccountsListPage();

    expect(screen.getByText("Accounts")).toBeTruthy();
    expect(screen.getByTitle("Create account")).toBeTruthy();
    expect(
      document.querySelectorAll('[data-slot="card"]').length,
    ).toBeGreaterThan(0);
  });

  it("renders fetched account summaries", async () => {
    mockDashboardRequests({
      accounts: [
        {
          id: 1,
          name: "IBKR",
          accountType: "BROKER",
          baseCurrency: "EUR",
          summaryStatus: "OK",
          cashTotalAmount: null,
          assetTotalAmount: null,
          totalAmount: "123.45000000",
          totalCurrency: "EUR",
        },
      ],
      portfolio: {
        ...defaultPortfolio,
        totalValueAmount: "123.45000000",
        accountTotals: [
          {
            id: 1,
            name: "IBKR",
            accountType: "BROKER",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: "123.45000000",
            totalCurrency: "EUR",
          },
        ],
        fxLastUpdated: "2026-03-22 10:00:00",
      },
    });

    renderAccountsListPage();

    expect(await screen.findByText("IBKR")).toBeTruthy();
    expect(screen.getByText(/broker/i)).toBeTruthy();
    expect(screen.getAllByText(/EUR/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("123.45 EUR").length).toBe(1);
    expect(
      screen.getByRole("link", { name: /IBKR/ }).getAttribute("href"),
    ).toBe("/accounts/1");
  });

  it("renders crypto account with Crypto label", async () => {
    mockDashboardRequests({
      accounts: [
        {
          id: 2,
          name: "Kraken",
          accountType: "CRYPTO",
          baseCurrency: "EUR",
          summaryStatus: "OK",
          cashTotalAmount: null,
          assetTotalAmount: null,
          totalAmount: "0.00000000",
          totalCurrency: "EUR",
        },
      ],
    });

    renderAccountsListPage();

    expect(await screen.findByText("Kraken")).toBeTruthy();
    expect(screen.getByText(/crypto/i)).toBeTruthy();
  });

  it("renders conversion unavailable when the backend summary cannot be calculated", async () => {
    mockDashboardRequests({
      accounts: [
        {
          id: 1,
          name: "IBKR",
          accountType: "BROKER",
          baseCurrency: "EUR",
          summaryStatus: "CONVERSION_UNAVAILABLE",
          cashTotalAmount: null,
          assetTotalAmount: null,
          totalAmount: null,
          totalCurrency: null,
        },
      ],
    });

    renderAccountsListPage();

    expect(await screen.findByText("Unavailable")).toBeTruthy();
  });

  it("renders the empty state when no accounts exist", async () => {
    mockDashboardRequests({ accounts: [] });

    renderAccountsListPage();

    expect(await screen.findByText("No accounts yet")).toBeTruthy();
  });

  it("renders an error state and retries the request", async () => {
    let attempt = 0;

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";

      attempt += 1;

      if (attempt <= 2) {
        return Promise.reject(new Error("network error"));
      }

      if (query.includes("accounts {")) {
        return gqlResponse({
          accounts: [
            {
              id: 1,
              name: "Main Bank",
              accountType: "BANK",
              baseCurrency: "USD",
              summaryStatus: "OK",
              cashTotalAmount: null,
              assetTotalAmount: null,
              totalAmount: "50.00000000",
              totalCurrency: "USD",
            },
          ],
        });
      }

      if (query.includes("portfolio {")) {
        return gqlResponse({ portfolio: defaultPortfolio });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountsListPage();

    expect(await screen.findByText("Could not load accounts")).toBeTruthy();

    fireEvent.click(screen.getByText("Retry"));

    await waitFor(() => {
      expect(screen.getByText("Main Bank")).toBeTruthy();
    });
  });

  it("links to account detail and account creation routes", async () => {
    mockDashboardRequests({
      accounts: [
        {
          id: 7,
          name: "IBKR",
          accountType: "BROKER",
          baseCurrency: "EUR",
          summaryStatus: "OK",
          cashTotalAmount: null,
          assetTotalAmount: null,
          totalAmount: "1.00000000",
          totalCurrency: "EUR",
        },
      ],
    });

    renderAccountsListPage();

    expect(
      (
        await screen.findByRole("link", {
          name: /IBKR/,
        })
      ).getAttribute("href"),
    ).toBe("/accounts/7");
    expect(
      screen.getByRole("link", { name: "Create account" }).getAttribute("href"),
    ).toBe("/accounts/new");
  });

  it("masks account totals when hidden mode is enabled", async () => {
    window.localStorage.setItem("ui.hide_values", "true");

    mockDashboardRequests({
      accounts: [
        {
          id: 1,
          name: "IBKR",
          accountType: "BROKER",
          baseCurrency: "EUR",
          summaryStatus: "OK",
          cashTotalAmount: null,
          assetTotalAmount: null,
          totalAmount: "123.45000000",
          totalCurrency: "EUR",
        },
      ],
    });

    renderAccountsListPage();

    const accountLink = await screen.findByRole("link", {
      name: /IBKR/,
    });
    const maskedAmounts = screen.getAllByText("•••• EUR");

    expect(maskedAmounts.length).toBe(1);
    expect(screen.queryByText("123.45 EUR")).toBeNull();
    expect(accountLink.textContent).toContain("•••• EUR");
    expect(accountLink.textContent).not.toContain("123.45 EUR");
    expect(maskedAmounts[0].className).toContain("tabular-nums");
  });
});
