import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";

import {
  getAssistantModelsApiUrl,
  getAssistantSelectedModelApiUrl,
  getAssistantThreadsApiUrl,
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
  });

  it("shows unavailable when the health request returns a non-success status", async () => {
    mockGqlAndHealth(503);

    renderApp(["/accounts"]);

    expect(await screen.findByTitle("Backend: unavailable")).toBeTruthy();
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
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeTruthy();
    expect(screen.getByTitle("Backend: checking")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Open assistant chat" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Portfolio" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Accounts" })).toBeTruthy();
  });

  it("opens the assistant popup from the shell header", async () => {
    mockGqlAndHealth(200);

    renderApp(["/accounts"]);

    expect(await screen.findByTitle("Backend: connected")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Open assistant chat" }));

    expect(await screen.findByRole("dialog")).toBeTruthy();
    expect(screen.getByText("Popup chat entrypoint for quick questions inside the app.")).toBeTruthy();
    expect(await screen.findByRole("combobox", { name: "Assistant model" })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Assistant message" })).toBeTruthy();
  });

  it("updates the assistant model from the popup selector", async () => {
    mockGqlAndHealth(200);

    renderApp(["/accounts"]);

    fireEvent.click(await screen.findByRole("button", { name: "Open assistant chat" }));

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
    expect(await screen.findByText("Active model: gpt-4.1-mini")).toBeTruthy();
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

    const totalAmounts = await screen.findAllByText("153.70 EUR");
    const totalAmount = totalAmounts[0];
    const initialWidth = totalAmount.getAttribute("style");
    expect(screen.getByText("103.70 EUR")).toBeTruthy();

    fireEvent.click(
      screen.getByRole("button", { name: "Hide financial values" }),
    );

    expect(await screen.findAllByText("•••• EUR")).toHaveLength(3);
    expect(screen.queryByText("153.70 EUR")).toBeNull();
    expect(screen.queryByText("103.70 EUR")).toBeNull();
    expect(window.localStorage.getItem("ui.hide_values")).toBe("true");
    expect(screen.getAllByText("•••• EUR")[0].getAttribute("style")).toBe(
      initialWidth,
    );

    view.unmount();
    renderApp(["/portfolio"]);

    expect(await screen.findAllByText("•••• EUR")).toHaveLength(3);
    expect(screen.getByRole("button", { name: "Show financial values" })).toBeTruthy();
    expect(screen.queryByText("153.70 EUR")).toBeNull();
  });
});
