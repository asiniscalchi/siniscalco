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

import { UiStateProvider } from "@/lib/ui-state-provider";

import { PortfolioPage } from ".";

function createTestClient() {
  return new ApolloClient({ link: new HttpLink({ uri: "http://localhost/graphql" }), cache: new InMemoryCache() });
}

function renderPortfolioPage() {
  return render(
    <ApolloProvider client={createTestClient()}>
      <UiStateProvider>
        <MemoryRouter>
          <PortfolioPage />
        </MemoryRouter>
      </UiStateProvider>
    </ApolloProvider>,
  );
}

const defaultFxRates = {
  targetCurrency: "EUR",
  rates: [],
  lastUpdated: null,
  refreshStatus: "AVAILABLE",
  refreshError: null,
};

function gqlResponse(data: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify({ data }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

function mockPortfolioRequest(summary: unknown) {
  vi.mocked(fetch).mockImplementation((_input, init) => {
    const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
    const query = body?.query ?? "";

    if (query.includes("portfolio")) {
      return gqlResponse({ portfolio: summary });
    }

    if (query.includes("fxRates")) {
      return gqlResponse({ fxRates: defaultFxRates });
    }

    throw new Error(`Unhandled GQL query: ${query}`);
  });
}

describe("PortfolioPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows a loading state before the portfolio resolves", () => {
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}));

    renderPortfolioPage();

    expect(
      document.querySelectorAll('[data-slot="card"]').length,
    ).toBeGreaterThan(0);
  });

  it("renders the portfolio overview when cash data exists", async () => {
    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "OK",
      totalValueAmount: "153.70000000",
      accountTotals: [
        {
          id: 1,
          name: "IBKR",
          accountType: "BROKER",
          summaryStatus: "OK",
          totalAmount: "103.70000000",
          totalCurrency: "EUR",
        },
        {
          id: 2,
          name: "Main Bank",
          accountType: "BANK",
          summaryStatus: "OK",
          totalAmount: "50.00000000",
          totalCurrency: "EUR",
        },
      ],
      cashByCurrency: [
        { currency: "EUR", amount: "50.00000000", convertedAmount: "50.00000000" },
        { currency: "GBP", amount: "10.00000000", convertedAmount: "11.70000000" },
        { currency: "USD", amount: "100.00000000", convertedAmount: "92.00000000" },
      ],
      fxLastUpdated: "2026-03-22 11:30:00",
      fxRefreshStatus: "AVAILABLE",
      fxRefreshError: null,
      allocationTotals: [{ label: "Cash", amount: "153.70000000" }],
      allocationIsPartial: false,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    expect((await screen.findAllByText("153.70 EUR")).length).toBeGreaterThan(0);
    expect(screen.getByText("Cash By Account")).toBeTruthy();
    expect(screen.getByText("103.70 EUR")).toBeTruthy();
    expect(screen.getByText("50.00 EUR")).toBeTruthy();
    expect(screen.getByText("Last FX update: 2026-03-22 11:30")).toBeTruthy();
  });

  it("renders the empty state when no cash balances exist", async () => {
    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "OK",
      totalValueAmount: "0.00000000",
      accountTotals: [
        {
          id: 1,
          name: "IBKR",
          accountType: "BROKER",
          summaryStatus: "OK",
          totalAmount: "0.00000000",
          totalCurrency: "EUR",
        },
      ],
      cashByCurrency: [],
      fxLastUpdated: null,
      fxRefreshStatus: "UNAVAILABLE",
      fxRefreshError: "FX refresh unavailable: no successful refresh has completed",
      allocationTotals: [],
      allocationIsPartial: false,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    expect(await screen.findByText("No portfolio cash data yet")).toBeTruthy();
  });

  it("renders conversion unavailable while keeping original cash balances visible", async () => {
    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "CONVERSION_UNAVAILABLE",
      totalValueAmount: null,
      accountTotals: [
        {
          id: 1,
          name: "IBKR",
          accountType: "BROKER",
          summaryStatus: "CONVERSION_UNAVAILABLE",
          totalAmount: null,
          totalCurrency: "EUR",
        },
      ],
      cashByCurrency: [
        { currency: "GBP", amount: "10.00000000", convertedAmount: null },
        { currency: "USD", amount: "100.00000000", convertedAmount: null },
      ],
      fxLastUpdated: "2026-03-22 10:00:00",
      fxRefreshStatus: "UNAVAILABLE",
      fxRefreshError: "FX refresh unavailable: provider returned status 500",
      allocationTotals: [],
      allocationIsPartial: true,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    expect(await screen.findAllByText("Conversion unavailable")).toHaveLength(2);
    expect(screen.getByText("Conversion data unavailable")).toBeTruthy();
    expect(screen.getByText("FX refresh unavailable")).toBeTruthy();
    expect(
      screen.getByText("FX refresh unavailable: provider returned status 500"),
    ).toBeTruthy();
  });

  it("renders an error state and retries the request", async () => {
    let attempt = 0;

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";

      if (query.includes("portfolio")) {
        attempt += 1;

        if (attempt === 1) {
          return Promise.reject(new Error("network error"));
        }

        return gqlResponse({
          portfolio: {
            displayCurrency: "EUR",
            totalValueStatus: "OK",
            totalValueAmount: "1.00000000",
            accountTotals: [],
            cashByCurrency: [{ currency: "EUR", amount: "1.00000000", convertedAmount: "1.00000000" }],
            fxLastUpdated: null,
            fxRefreshStatus: "AVAILABLE",
            fxRefreshError: null,
            allocationTotals: [{ label: "Cash", amount: "1.00000000" }],
            allocationIsPartial: false,
            holdings: [],
            holdingsIsPartial: false,
          },
        });
      }

      if (query.includes("fxRates")) {
        return gqlResponse({ fxRates: defaultFxRates });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderPortfolioPage();

    expect(await screen.findByText("Could not load portfolio")).toBeTruthy();

    fireEvent.click(screen.getByText("Retry"));

    await waitFor(() => {
      expect(screen.getAllByText("1.00 EUR").length).toBeGreaterThan(0);
    });
  });

  it("masks portfolio values when hidden mode is enabled", async () => {
    window.localStorage.setItem("ui.hide_values", "true");

    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "OK",
      totalValueAmount: "153.70000000",
      accountTotals: [
        {
          id: 1,
          name: "IBKR",
          accountType: "BROKER",
          summaryStatus: "OK",
          totalAmount: "103.70000000",
          totalCurrency: "EUR",
        },
      ],
      cashByCurrency: [
        { currency: "USD", amount: "100.00000000", convertedAmount: "92.00000000" },
      ],
      fxLastUpdated: "2026-03-22 11:30:00",
      fxRefreshStatus: "AVAILABLE",
      fxRefreshError: null,
      allocationTotals: [{ label: "Cash", amount: "92.00000000" }],
      allocationIsPartial: false,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    expect(await screen.findAllByText("•••• EUR")).toHaveLength(3);
  });

  it("handles missing currency conversion values without crashing", async () => {
    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "OK",
      totalValueAmount: "10.00000000",
      accountTotals: [
        {
          id: 1,
          name: "Empty Account",
          accountType: "BANK",
          summaryStatus: "CONVERSION_UNAVAILABLE",
          totalAmount: null,
          totalCurrency: "EUR",
        },
      ],
      cashByCurrency: [
        { currency: "JPY", amount: "1000.00000000", convertedAmount: null },
      ],
      fxLastUpdated: null,
      fxRefreshStatus: "AVAILABLE",
      fxRefreshError: null,
      allocationTotals: [],
      allocationIsPartial: true,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    expect(await screen.findByText("Conversion unavailable")).toBeTruthy();
    expect(screen.queryByText("Conversion data unavailable")).toBeNull();
  });

  it("renders the allocation card with slices and labels", async () => {
    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "OK",
      totalValueAmount: "300.00000000",
      accountTotals: [],
      cashByCurrency: [
        { currency: "EUR", amount: "100.00000000", convertedAmount: "100.00000000" },
      ],
      fxLastUpdated: null,
      fxRefreshStatus: "AVAILABLE",
      fxRefreshError: null,
      allocationTotals: [
        { label: "Stock", amount: "200.00000000" },
        { label: "Cash", amount: "100.00000000" },
      ],
      allocationIsPartial: false,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    expect(await screen.findByText("Allocation by asset class")).toBeTruthy();
    expect(screen.getByText("Stock")).toBeTruthy();
    expect(screen.getByText("Cash")).toBeTruthy();
    expect(screen.getByText("200.00 EUR")).toBeTruthy();
    expect(screen.getByText("100.00 EUR")).toBeTruthy();
    expect(screen.getByText("66.7%")).toBeTruthy();
    expect(screen.getByText("33.3%")).toBeTruthy();
  });

  it("shows the partial banner when allocationIsPartial is true", async () => {
    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "OK",
      totalValueAmount: "100.00000000",
      accountTotals: [],
      cashByCurrency: [
        { currency: "EUR", amount: "100.00000000", convertedAmount: "100.00000000" },
      ],
      fxLastUpdated: null,
      fxRefreshStatus: "AVAILABLE",
      fxRefreshError: null,
      allocationTotals: [{ label: "Cash", amount: "100.00000000" }],
      allocationIsPartial: true,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    expect(
      await screen.findByText(
        "Allocation incomplete: some assets could not be valued.",
      ),
    ).toBeTruthy();
  });

  it("shows no-data message when allocationTotals is empty and not partial", async () => {
    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "OK",
      totalValueAmount: "100.00000000",
      accountTotals: [],
      cashByCurrency: [
        { currency: "EUR", amount: "100.00000000", convertedAmount: "100.00000000" },
      ],
      fxLastUpdated: null,
      fxRefreshStatus: "AVAILABLE",
      fxRefreshError: null,
      allocationTotals: [],
      allocationIsPartial: false,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    expect(await screen.findByText("No allocation data available.")).toBeTruthy();
  });

  it("masks allocation amounts and percentages in privacy mode", async () => {
    window.localStorage.setItem("ui.hide_values", "true");

    mockPortfolioRequest({
      displayCurrency: "EUR",
      totalValueStatus: "OK",
      totalValueAmount: "300.00000000",
      accountTotals: [],
      cashByCurrency: [
        { currency: "EUR", amount: "100.00000000", convertedAmount: "100.00000000" },
      ],
      fxLastUpdated: null,
      fxRefreshStatus: "AVAILABLE",
      fxRefreshError: null,
      allocationTotals: [
        { label: "Stock", amount: "200.00000000" },
        { label: "Cash", amount: "100.00000000" },
      ],
      allocationIsPartial: false,
      holdings: [],
      holdingsIsPartial: false,
    });

    renderPortfolioPage();

    await screen.findByText("Allocation by asset class");
    expect(screen.queryByText("200.00 EUR")).toBeNull();
    expect(screen.queryByText("66.7%")).toBeNull();
    expect(screen.getAllByText("•••%").length).toBeGreaterThan(0);
  });
});
