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

import {
  getApiBaseUrl,
  getAssistantModelsApiUrl,
  getAssistantSelectedModelApiUrl,
  getAssistantThreadsApiUrl,
  getVersionApiUrl,
} from "@/lib/env";
import { UiStateProvider } from "@/lib/ui-state-provider";
import { ResizeObserverMock } from "@/test/browser-mocks";
import App from "./App";

function createTestClient() {
  return new ApolloClient({ link: new HttpLink({ uri: "http://localhost/graphql" }), cache: new InMemoryCache() });
}

function renderApp(initialEntries: string[]) {
  return render(
    <ApolloProvider client={createTestClient()}>
      <UiStateProvider>
        <MemoryRouter initialEntries={initialEntries}>
          <App />
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

const emptyPortfolio = {
  displayCurrency: "EUR",
  totalValueStatus: "OK",
  totalValueAmount: "0.00000000",
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

const emptyFxRates = {
  targetCurrency: "EUR",
  rates: [],
  lastUpdated: null,
  refreshStatus: "AVAILABLE",
  refreshError: null,
};

function mockGqlAndHealth(
  healthStatus: number,
  overrides?: (query: string) => Promise<Response> | null,
) {
  vi.mocked(fetch).mockImplementation((input, init) => {
    const url = String(input);

    if (url.endsWith("/health")) {
      return Promise.resolve(new Response(null, { status: healthStatus }));
    }

    if (url === getVersionApiUrl()) {
      return Promise.resolve(
        new Response(JSON.stringify({ version: "test-sha" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );
    }

    if (url === getAssistantThreadsApiUrl()) {
      return Promise.resolve(
        new Response(JSON.stringify([]), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );
    }

    if (url === getAssistantModelsApiUrl()) {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            models: ["gpt-4o-mini", "gpt-4.1-mini"],
            selected_model: "gpt-4o-mini",
            openai_enabled: true,
            last_refreshed_at: "2026-03-29T12:00:00Z",
            refresh_error: null,
          }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          },
        ),
      );
    }

    if (url === getAssistantSelectedModelApiUrl()) {
      const body = init?.body
        ? (JSON.parse(String(init.body)) as { model: string })
        : { model: "gpt-4o-mini" };
      return Promise.resolve(
        new Response(
          JSON.stringify({
            models: ["gpt-4o-mini", "gpt-4.1-mini"],
            selected_model: body.model,
            openai_enabled: true,
            last_refreshed_at: "2026-03-29T12:00:00Z",
            refresh_error: null,
          }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          },
        ),
      );
    }

    const body = init?.body
      ? (JSON.parse(String(init.body)) as { query: string })
      : null;
    const query = body?.query ?? "";

    if (overrides) {
      const result = overrides(query);
      if (result !== null) return result;
    }

    if (query.includes("accounts {")) return gqlResponse({ accounts: [] });
    if (query.includes("portfolio {")) return gqlResponse({ portfolio: emptyPortfolio });
    if (query.includes("fxRates {")) return gqlResponse({ fxRates: emptyFxRates });
    if (query.includes("currencies")) return gqlResponse({ currencies: ["EUR", "USD"] });
    if (query.includes("account(")) return gqlResponse({ account: null });
    if (query.includes("accountPositions")) return gqlResponse({ accountPositions: [] });
    if (query.includes("assets {")) return gqlResponse({ assets: [] });
    if (query.includes("todos {")) return gqlResponse({ todos: [] });
    if (query.includes("transactions {")) return gqlResponse({ transactions: [] });

    throw new Error(`Unhandled GQL query: ${query}`);
  });
}

describe("App shell", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);
    window.HTMLElement.prototype.scrollTo = vi.fn();
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("requests health on shell mount and shows connected from the response status", async () => {
    mockGqlAndHealth(200);

    renderApp(["/accounts"]);

    expect(fetch).toHaveBeenCalledWith(expect.stringMatching(/\/health$/));
    expect(await screen.findByTitle("Backend: connected")).toBeTruthy();
    expect(screen.queryByText(getApiBaseUrl())).toBeNull();
  });

  it("shows unavailable when the health request returns a non-success status", async () => {
    mockGqlAndHealth(503);

    renderApp(["/accounts"]);

    expect(await screen.findByTitle("Backend: unavailable")).toBeTruthy();
    expect(screen.getByText(getApiBaseUrl())).toBeTruthy();
  });

  it("shows connected when the health request returns another successful status", async () => {
    mockGqlAndHealth(204);

    renderApp(["/accounts"]);

    expect(await screen.findByTitle("Backend: connected")).toBeTruthy();
  });

  it("renders the shell while page content and health are still loading", () => {
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}));

    renderApp(["/accounts"]);

    expect(screen.getByLabelText("Siniscalco")).toBeTruthy();
    expect(screen.queryByText(getApiBaseUrl())).toBeNull();
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeTruthy();
    expect(screen.getByTitle("Backend: checking")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Open assistant chat" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Portfolio" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Accounts" })).toBeTruthy();
    expect(screen.queryByRole("link", { name: "Transfers" })).toBeNull();
  });

  it("shows the open todo count in the primary navigation", async () => {
    mockGqlAndHealth(200, (query) => {
      if (query.includes("todos {")) {
        return gqlResponse({
          todos: [
            { id: 1, completed: false },
            { id: 2, completed: true },
            { id: 3, completed: false },
          ],
        });
      }
      return null;
    });

    renderApp(["/accounts"]);

    const todosLink = await screen.findByRole("link", { name: /Todos/ });
    expect(within(todosLink).getByText("2")).toBeTruthy();
  });

  it("shows an asset daily gain ticker below the header and keeps percentages visible when values are hidden", async () => {
    mockGqlAndHealth(200, (query) => {
      if (query.includes("assets {")) {
        return gqlResponse({
          assets: [
            {
              id: 1,
              symbol: "AAPL",
              name: "Apple Inc.",
              assetType: "STOCK",
              quoteSymbol: "AAPL",
              isin: "US0378331005",
              quoteSourceSymbol: "AAPL",
              quoteSourceProvider: "yahoo",
              quoteSourceLastSuccessAt: "2026-03-24T14:30:00Z",
              currentPrice: "189.326789",
              currentPriceCurrency: "USD",
              currentPriceAsOf: "2026-03-24T14:30:00Z",
              totalQuantity: "10.5",
              avgCostBasis: null,
              avgCostBasisCurrency: null,
              previousClose: "180.000000",
              previousCloseCurrency: "USD",
              convertedTotalValue: "1250.500000",
              convertedTotalValueCurrency: "EUR",
            },
            {
              id: 2,
              symbol: "BTC",
              name: "Bitcoin",
              assetType: "CRYPTO",
              quoteSymbol: "BTC/USD",
              isin: null,
              quoteSourceSymbol: null,
              quoteSourceProvider: null,
              quoteSourceLastSuccessAt: null,
              currentPrice: "90.000000",
              currentPriceCurrency: "USD",
              currentPriceAsOf: null,
              totalQuantity: null,
              avgCostBasis: null,
              avgCostBasisCurrency: null,
              previousClose: "100.000000",
              previousCloseCurrency: "USD",
              convertedTotalValue: "420.000000",
              convertedTotalValueCurrency: "EUR",
            },
            {
              id: 3,
              symbol: "MSFT",
              name: "Microsoft",
              assetType: "STOCK",
              quoteSymbol: "MSFT",
              isin: "US5949181045",
              quoteSourceSymbol: "MSFT",
              quoteSourceProvider: "yahoo",
              quoteSourceLastSuccessAt: "2026-03-24T14:30:00Z",
              currentPrice: "300.000000",
              currentPriceCurrency: "USD",
              currentPriceAsOf: "2026-03-24T14:30:00Z",
              totalQuantity: "1",
              avgCostBasis: null,
              avgCostBasisCurrency: null,
              previousClose: "300.000000",
              previousCloseCurrency: "USD",
              convertedTotalValue: "300.000000",
              convertedTotalValueCurrency: "EUR",
            },
          ],
        });
      }

      return null;
    });

    renderApp(["/accounts"]);

    expect(await screen.findByTestId("asset-value-ticker")).toBeTruthy();
    expect(screen.getAllByText("AAPL").length).toBeGreaterThan(0);
    expect(screen.getAllByText("+5.18%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("-10.00%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("MSFT").length).toBeGreaterThan(0);
    expect(screen.getAllByText("0.00%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("AAPL")[0].className).toContain("text-green");
    expect(screen.getAllByText("+5.18%")[0].className).toContain("text-green");
    expect(screen.getAllByText("BTC")[0].className).toContain("text-red");
    expect(screen.getAllByText("-10.00%")[0].className).toContain("text-red");
    expect(screen.getAllByText("MSFT")[0].className).toContain("text-muted");
    expect(screen.getAllByText("0.00%")[0].className).toContain("text-muted");

    fireEvent.click(
      screen.getByRole("button", { name: "Hide financial values" }),
    );

    // Percentages are not sensitive — they must remain visible even when values are hidden
    await waitFor(() => {
      expect(screen.getAllByText("+5.18%").length).toBeGreaterThan(0);
    });
    expect(screen.getAllByText("-10.00%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("0.00%").length).toBeGreaterThan(0);
  });

  it("opens the assistant popup from the shell header", async () => {
    mockGqlAndHealth(200);

    renderApp(["/accounts"]);

    expect(await screen.findByTitle("Backend: connected")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Open assistant chat" }));

    expect(await screen.findByRole("dialog")).toBeTruthy();
    expect(screen.getByTestId("assistant-panel").className).toContain(
      "sm:w-[90dvw]",
    );
    expect(screen.getByTestId("assistant-panel").className).toContain(
      "sm:h-[90dvh]",
    );
    expect(screen.getByRole("textbox", { name: "Assistant message" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Show chat history" }));
    expect(await screen.findByText("Chats")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Show settings" }));
    expect(await screen.findByRole("combobox", { name: "Assistant model" })).toBeTruthy();
  });

  it("updates the assistant model from the popup selector", async () => {
    mockGqlAndHealth(200);

    renderApp(["/accounts"]);

    fireEvent.click(await screen.findByRole("button", { name: "Open assistant chat" }));
    fireEvent.click(await screen.findByRole("button", { name: "Show settings" }));

    const modelSelect = await screen.findByRole("combobox", {
      name: "Assistant model",
    });
    fireEvent.change(modelSelect, { target: { value: "gpt-4.1-mini" } });

    await waitFor(() => {
      expect(vi.mocked(fetch).mock.calls).toContainEqual([
        getAssistantSelectedModelApiUrl(),
        expect.objectContaining({
          method: "PUT",
        }),
      ]);
    });
  });

  it("keeps the shell rendered while navigating between wrapped routes", async () => {
    mockGqlAndHealth(200, (query) => {
      if (query.includes("accounts {")) {
        return gqlResponse({
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
      }
      if (query.includes("portfolio {")) {
        return gqlResponse({
          portfolio: {
            ...emptyPortfolio,
            totalValueAmount: "1.00000000",
            accountTotals: [
              {
                id: 7,
                name: "IBKR",
                accountType: "BROKER",
                summaryStatus: "OK",
                cashTotalAmount: null,
                assetTotalAmount: null,
                totalAmount: "1.00000000",
                totalCurrency: "EUR",
              },
            ],
            cashByCurrency: [{ currency: "EUR", amount: "1.00000000", convertedAmount: null }],
          },
        });
      }
      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: 7,
            name: "IBKR",
            accountType: "BROKER",
            baseCurrency: "EUR",
            summaryStatus: "OK",
            createdAt: "2026-03-22 00:00:00",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: [],
          },
        });
      }
      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }
      return null;
    });

    renderApp(["/accounts"]);

    const portfolioLink = screen.getByRole("link", { name: "Portfolio" });
    const accountsLink = screen.getByRole("link", { name: "Accounts" });
    expect(portfolioLink.getAttribute("aria-current")).toBeNull();
    expect(accountsLink.getAttribute("aria-current")).toBe("page");
    expect(accountsLink.className).toContain("border-foreground");

    expect(await screen.findByText("IBKR")).toBeTruthy();
    expect(screen.getByTitle("Backend: connected")).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Create account" }));

    expect(await screen.findByText("New Account")).toBeTruthy();
    expect(screen.getByLabelText("Siniscalco")).toBeTruthy();
    expect(screen.getByTitle("Backend: connected")).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: "Accounts" })
        .getAttribute("aria-current"),
    ).toBe("page");

    fireEvent.click(screen.getByRole("link", { name: "Cancel" }));

    expect(await screen.findByText("IBKR")).toBeTruthy();

    fireEvent.click(
      screen.getByRole("link", { name: /IBKR/ }),
    );

    expect(await screen.findByText("Account Summary")).toBeTruthy();
    expect(screen.getByLabelText("Siniscalco")).toBeTruthy();
    expect(screen.getByTitle("Backend: connected")).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: "Accounts" })
        .getAttribute("aria-current"),
    ).toBe("page");
  });

  it("toggles hidden values, persists the choice, and keeps amount width stable", async () => {
    mockGqlAndHealth(200, (query) => {
      if (query.includes("portfolio {")) {
        return gqlResponse({
          portfolio: {
            displayCurrency: "EUR",
            totalValueStatus: "OK",
            totalValueAmount: "153.70000000",
            accountTotals: [
              {
                id: 1,
                name: "IBKR",
                accountType: "BROKER",
                summaryStatus: "OK",
                cashTotalAmount: null,
                assetTotalAmount: null,
                totalAmount: "103.70000000",
                totalCurrency: "EUR",
              },
            ],
            cashByCurrency: [
              {
                currency: "USD",
                amount: "100.00000000",
                convertedAmount: "92.00000000",
              },
            ],
            fxLastUpdated: null,
            fxRefreshStatus: "AVAILABLE",
            fxRefreshError: null,
            allocationTotals: [{ label: "Cash", amount: "92.00000000" }],
            allocationIsPartial: false,
            holdings: [],
            holdingsIsPartial: false,
          },
        });
      }
      if (query.includes("fxRates {")) {
        return gqlResponse({ fxRates: emptyFxRates });
      }
      return null;
    });

    const view = renderApp(["/portfolio"]);

    const totalAmounts = await screen.findAllByText("€153.70");
    const totalAmount = totalAmounts[0];
    const initialWidth = totalAmount.getAttribute("style");
    expect(screen.getByText("€103.70")).toBeTruthy();

    fireEvent.click(
      screen.getByRole("button", { name: "Hide financial values" }),
    );

    expect(await screen.findAllByText("€••••")).toHaveLength(3);
    expect(screen.queryByText("€153.70")).toBeNull();
    expect(screen.queryByText("€103.70")).toBeNull();
    expect(window.localStorage.getItem("ui.hide_values")).toBe("true");
    expect(screen.getAllByText("€••••")[0].getAttribute("style")).toBe(
      initialWidth,
    );

    view.unmount();
    renderApp(["/portfolio"]);

    expect(await screen.findAllByText("€••••")).toHaveLength(3);
    expect(screen.getByRole("button", { name: "Show financial values" })).toBeTruthy();
    expect(screen.queryByText("€153.70")).toBeNull();
  });
});
