import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";

import { UiStateProvider } from "@/lib/ui-state-provider";
import { AccountDetailPage } from ".";

function gqlResponse(data: unknown, status = 200) {
  return Promise.resolve(
    new Response(JSON.stringify({ data }), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

function gqlErrorResponse(message: string) {
  return Promise.resolve(
    new Response(
      JSON.stringify({
        data: null,
        errors: [{ message }],
      }),
      {
        status: 200,
        headers: { "Content-Type": "application/json" },
      },
    ),
  );
}

function mockAccountDetailFetch(
  handler: (query: string, variables: Record<string, unknown>) => Promise<Response> | Response,
) {
  vi.mocked(fetch).mockImplementation((_input, init) => {
    const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
    const query = body?.query ?? "";
    const variables = body?.variables ?? {};

    if (query.includes("currencies")) {
      return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
    }

    if (query.includes("assets") && !query.includes("account")) {
      return gqlResponse({ assets: [] });
    }

    if (query.includes("accountPositions")) {
      return gqlResponse({ accountPositions: [] });
    }

    return Promise.resolve(handler(query, variables));
  });
}

function createTestClient() {
  return new ApolloClient({ link: new HttpLink({ uri: "http://localhost/graphql" }), cache: new InMemoryCache() });
}

function renderAccountDetailPage(initialEntry: string, routes?: ReactNode) {
  return render(
    <ApolloProvider client={createTestClient()}>
      <UiStateProvider>
        <MemoryRouter initialEntries={[initialEntry]}>
          <Routes>
            {routes ?? (
              <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
            )}
          </Routes>
        </MemoryRouter>
      </UiStateProvider>
    </ApolloProvider>,
  );
}

describe("AccountDetailPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("renders account detail with balances", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";
      const variables = body?.variables ?? {};

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: variables.id,
            name: "IBKR",
            accountType: "BROKER",
            baseCurrency: "EUR",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: [
              {
                currency: "USD",
                amount: "12.30000000",
                updatedAt: "2026-03-22 00:00:00",
              },
            ],
          },
        });
      }

      if (query.includes("accountPositions")) {
        return gqlResponse({ accountPositions: [] });
      }

      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }

      if (query.includes("fxRates")) {
        return gqlResponse({ fxRates: { targetCurrency: "EUR", rates: [], lastUpdated: null, refreshStatus: "AVAILABLE", refreshError: null } });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage("/accounts/7");

    expect(await screen.findByText("IBKR")).toBeTruthy();
    expect(screen.getByText("BROKER · base currency EUR")).toBeTruthy();
    expect(screen.getByText("12.30000000")).toBeTruthy();
  });

  it("renders account assets when the account has positions", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: 7,
            name: "IBKR",
            accountType: "BROKER",
            baseCurrency: "EUR",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: [],
          },
        });
      }

      if (query.includes("accountPositions")) {
        return gqlResponse({
          accountPositions: [
            {
              accountId: 7,
              assetId: 3,
              quantity: "2.500000",
            },
          ],
        });
      }

      if (query.includes("assets")) {
        return gqlResponse({
          assets: [
            {
              id: 3,
              symbol: "BTC",
              name: "Bitcoin",
              assetType: "CRYPTO",
              quoteSymbol: "BTC-USD",
              isin: null,
              currentPrice: "90000.00",
              currentPriceCurrency: "USD",
              currentPriceAsOf: "2026-03-22T00:00:00Z",
              totalQuantity: null,
            },
          ],
        });
      }

      if (query.includes("fxRates")) {
        return gqlResponse({
          fxRates: {
            targetCurrency: "EUR",
            rates: [{ currency: "USD", rate: "0.92" }],
            lastUpdated: null,
            refreshStatus: "AVAILABLE",
            refreshError: null,
          },
        });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage("/accounts/7");

    expect(await screen.findByRole("heading", { name: "Assets" })).toBeTruthy();
    expect(screen.getAllByText("BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Bitcoin").length).toBeGreaterThan(0);
    expect(screen.getAllByText("2.5").length).toBeGreaterThan(0);
    // value = 2.5 * 90000 * (0.92 / 1) = 207,000 EUR
    expect(screen.getAllByText("207,000.00 EUR").length).toBeGreaterThan(0);
  });

  it("renders account detail with empty balances", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: 3,
            name: "Main Bank",
            accountType: "BANK",
            baseCurrency: "USD",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: [],
          },
        });
      }

      if (query.includes("accountPositions")) {
        return gqlResponse({ accountPositions: [] });
      }

      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }

      if (query.includes("fxRates")) {
        return gqlResponse({ fxRates: { targetCurrency: "USD", rates: [], lastUpdated: null, refreshStatus: "AVAILABLE", refreshError: null } });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage("/accounts/3");

    expect(await screen.findByText("Main Bank")).toBeTruthy();
    expect(screen.getByText("No balances yet")).toBeTruthy();
    expect(fetch).toHaveBeenCalled();
  });

  it("renders an error state and retries the request", async () => {
    let accountRequestCount = 0;

    mockAccountDetailFetch((query, variables) => {
      if (query.includes("account(")) {
        accountRequestCount += 1;

        if (accountRequestCount === 1) {
          return gqlErrorResponse("Account not found");
        }

        return gqlResponse({
          account: {
            id: variables.id,
            name: "Broker",
            accountType: "BROKER",
            baseCurrency: "EUR",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: [
              {
                currency: "EUR",
                amount: "100.00000000",
                updatedAt: "2026-03-22 00:00:00",
              },
            ],
          },
        });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage("/accounts/8");

    expect(await screen.findByText("Could not load account")).toBeTruthy();
    expect(screen.getByText("Account not found")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText("Broker")).toBeTruthy();
    expect(screen.getByText("100.00000000")).toBeTruthy();
  });

  it("upserts a balance from the account detail page", async () => {
    let saved = false;

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";
      const variables = body?.variables ?? {};

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: 9,
            name: "IBKR",
            accountType: "BROKER",
            baseCurrency: "EUR",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: saved
              ? [
                  {
                    currency: "USD",
                    amount: "42.50000000",
                    updatedAt: "2026-03-22 00:00:00",
                  },
                ]
              : [],
          },
        });
      }

      if (query.includes("accountPositions")) {
        return gqlResponse({ accountPositions: [] });
      }

      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }

      if (query.includes("fxRates")) {
        return gqlResponse({ fxRates: { targetCurrency: "EUR", rates: [], lastUpdated: null, refreshStatus: "AVAILABLE", refreshError: null } });
      }

      if (query.includes("upsertBalance")) {
        expect(variables).toEqual(
          expect.objectContaining({
            accountId: 9,
            input: { currency: "USD", amount: "42.5" },
          }),
        );
        saved = true;
        return gqlResponse({
          upsertBalance: {
            currency: "USD",
            amount: "42.50000000",
            updatedAt: "2026-03-22 00:00:00",
          },
        });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage("/accounts/9");

    expect(await screen.findByText("No balances yet")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Currency"), {
      target: { value: "USD" },
    });
    fireEvent.change(screen.getByLabelText("Amount"), {
      target: { value: "42.5" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save balance" }));

    expect(await screen.findByText("42.50000000")).toBeTruthy();
  });

  it("deletes a balance from the account detail page", async () => {
    let deleted = false;

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: 10,
            name: "Main Bank",
            accountType: "BANK",
            baseCurrency: "USD",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: deleted
              ? []
              : [
                  {
                    currency: "USD",
                    amount: "100.00000000",
                    updatedAt: "2026-03-22 00:00:00",
                  },
                ],
          },
        });
      }

      if (query.includes("accountPositions")) {
        return gqlResponse({ accountPositions: [] });
      }

      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }

      if (query.includes("fxRates")) {
        return gqlResponse({ fxRates: { targetCurrency: "USD", rates: [], lastUpdated: null, refreshStatus: "AVAILABLE", refreshError: null } });
      }

      if (query.includes("deleteBalance")) {
        deleted = true;
        return gqlResponse({ deleteBalance: true });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage("/accounts/10");

    expect(await screen.findByText("100.00000000")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    expect(await screen.findByText("No balances yet")).toBeTruthy();
  });

  it("deletes an account from the account detail page", async () => {
    mockAccountDetailFetch((query, variables) => {
      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: variables.id,
            name: "Broker Account",
            accountType: "BROKER",
            baseCurrency: "EUR",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: [],
          },
        });
      }

      if (query.includes("deleteAccount")) {
        return gqlResponse({ deleteAccount: true });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage(
      "/accounts/13",
      <>
        <Route path="/accounts" element={<div>Accounts Route</div>} />
        <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
      </>,
    );

    expect(await screen.findByText("Broker Account")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Delete account" }));

    expect(await screen.findByText("Accounts Route")).toBeTruthy();
  });

  it("renders an error when account deletion fails", async () => {
    mockAccountDetailFetch((query, variables) => {
      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: variables.id,
            name: "Checking",
            accountType: "BANK",
            baseCurrency: "USD",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: [],
          },
        });
      }

      if (query.includes("deleteAccount")) {
        return gqlErrorResponse("Could not delete account.");
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage("/accounts/14");

    expect(await screen.findByText("Checking")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Delete account" }));

    expect(await screen.findByText("Could not delete account.")).toBeTruthy();
  });

  it("resets the balance form when the loaded account changes", async () => {
    mockAccountDetailFetch((query, variables) => {
      if (query.includes("account(")) {
        const id = variables.id as number;
        if (id === 11) {
          return gqlResponse({
            account: {
              id: 11,
              name: "First Account",
              accountType: "BROKER",
              baseCurrency: "EUR",
              createdAt: "2026-03-22 00:00:00",
              summaryStatus: "OK",
              cashTotalAmount: null,
              assetTotalAmount: null,
              totalAmount: null,
              totalCurrency: null,
              balances: [],
            },
          });
        }

        if (id === 12) {
          return gqlResponse({
            account: {
              id: 12,
              name: "Second Account",
              accountType: "BANK",
              baseCurrency: "USD",
              createdAt: "2026-03-22 00:00:00",
              summaryStatus: "OK",
              cashTotalAmount: null,
              assetTotalAmount: null,
              totalAmount: null,
              totalCurrency: null,
              balances: [],
            },
          });
        }
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    const firstRender = renderAccountDetailPage("/accounts/11");

    expect(await screen.findByText("First Account")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Currency"), {
      target: { value: "GBP" },
    });
    fireEvent.change(screen.getByLabelText("Amount"), {
      target: { value: "99.5" },
    });

    firstRender.unmount();

    renderAccountDetailPage("/accounts/12");

    expect(await screen.findByText("Second Account")).toBeTruthy();
    expect((screen.getByLabelText("Currency") as HTMLSelectElement).value).toBe(
      "USD",
    );
    expect((screen.getByLabelText("Amount") as HTMLInputElement).value).toBe(
      "",
    );
  });

  it("masks account balances when hidden mode is enabled", async () => {
    window.localStorage.setItem("ui.hide_values", "true");

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("account(")) {
        return gqlResponse({
          account: {
            id: 7,
            name: "IBKR",
            accountType: "BROKER",
            baseCurrency: "EUR",
            createdAt: "2026-03-22 00:00:00",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: null,
            totalCurrency: null,
            balances: [
              {
                currency: "USD",
                amount: "12.30000000",
                updatedAt: "2026-03-22 00:00:00",
              },
            ],
          },
        });
      }

      if (query.includes("accountPositions")) {
        return gqlResponse({ accountPositions: [] });
      }

      if (query.includes("assets")) {
        return gqlResponse({ assets: [] });
      }

      if (query.includes("fxRates")) {
        return gqlResponse({ fxRates: { targetCurrency: "EUR", rates: [], lastUpdated: null, refreshStatus: "AVAILABLE", refreshError: null } });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    renderAccountDetailPage("/accounts/7");

    expect(await screen.findByText("IBKR")).toBeTruthy();
    expect(screen.getByText("••••")).toBeTruthy();
    expect(screen.queryByText("12.30000000")).toBeNull();
  });
});
