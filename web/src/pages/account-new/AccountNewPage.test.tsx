import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";

import { AccountNewPage } from ".";
import { AccountsListPage } from "../accounts-list";
import { UiStateProvider } from "@/lib/ui-state-provider";

function createTestClient() {
  return new ApolloClient({ link: new HttpLink({ uri: "http://localhost/graphql" }), cache: new InMemoryCache() });
}

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

describe("AccountNewPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("renders the account creation form", () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({ data: { currencies: ["CHF", "EUR", "GBP", "USD"] } }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    render(
      <ApolloProvider client={createTestClient()}>
        <MemoryRouter>
          <AccountNewPage />
        </MemoryRouter>
      </ApolloProvider>,
    );

    expect(screen.getByText("New Account")).toBeTruthy();
    expect(screen.getByLabelText("Name")).toBeTruthy();
    expect(screen.getByLabelText("Account type")).toBeTruthy();
    expect(screen.getByLabelText("Base currency")).toBeTruthy();
  });

  it("creates an account and returns to the accounts list route", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("createAccount")) {
        return gqlResponse({
          createAccount: {
            id: 12,
            name: "IBKR",
            accountType: "BROKER",
            baseCurrency: "EUR",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: "0.00000000",
            totalCurrency: "EUR",
            createdAt: "2026-03-22 00:00:00",
            balances: [],
          },
        });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    render(
      <ApolloProvider client={createTestClient()}>
        <MemoryRouter initialEntries={["/accounts/new"]}>
          <Routes>
            <Route path="/accounts/new" element={<AccountNewPage />} />
            <Route path="/accounts" element={<div>Accounts Route</div>} />
          </Routes>
        </MemoryRouter>
      </ApolloProvider>,
    );

    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "IBKR" },
    });
    fireEvent.change(screen.getByLabelText("Account type"), {
      target: { value: "broker" },
    });
    await screen.findByRole("option", { name: "CHF" });

    fireEvent.change(screen.getByLabelText("Base currency"), {
      target: { value: "EUR" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(await screen.findByText("Accounts Route")).toBeTruthy();
  });

  it("creates a crypto account", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string; variables?: Record<string, unknown> } : null;
      const query = body?.query ?? "";

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("createAccount")) {
        return gqlResponse({
          createAccount: {
            id: 13,
            name: "Kraken",
            accountType: "CRYPTO",
            baseCurrency: "EUR",
            summaryStatus: "OK",
            cashTotalAmount: null,
            assetTotalAmount: null,
            totalAmount: "0.00000000",
            totalCurrency: "EUR",
            createdAt: "2026-03-22 00:00:00",
            balances: [],
          },
        });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    render(
      <ApolloProvider client={createTestClient()}>
        <MemoryRouter initialEntries={["/accounts/new"]}>
          <Routes>
            <Route path="/accounts/new" element={<AccountNewPage />} />
            <Route path="/accounts" element={<div>Accounts Route</div>} />
          </Routes>
        </MemoryRouter>
      </ApolloProvider>,
    );

    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "Kraken" },
    });
    fireEvent.change(screen.getByLabelText("Account type"), {
      target: { value: "crypto" },
    });
    await screen.findByRole("option", { name: "CHF" });

    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(await screen.findByText("Accounts Route")).toBeTruthy();
  });

  it("shows an API error when account creation fails", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("createAccount")) {
        return gqlErrorResponse("currency must be one of: EUR, USD, GBP, CHF");
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    render(
      <ApolloProvider client={createTestClient()}>
        <MemoryRouter>
          <AccountNewPage />
        </MemoryRouter>
      </ApolloProvider>,
    );

    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "IBKR" },
    });
    await screen.findByRole("option", { name: "CHF" });
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(
      await screen.findByText("currency must be one of: EUR, USD, GBP, CHF"),
    ).toBeTruthy();
  });

  it("shows the new account in the accounts list after creation", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";

      if (query.includes("currencies")) {
        return gqlResponse({ currencies: ["CHF", "EUR", "GBP", "USD"] });
      }

      if (query.includes("createAccount")) {
        return gqlResponse({
          createAccount: {
            id: 99,
            name: "New Broker",
            accountType: "BROKER",
            baseCurrency: "EUR",
          },
        });
      }

      if (query.includes("accounts {")) {
        return gqlResponse({
          accounts: [
            {
              id: 99,
              name: "New Broker",
              accountType: "BROKER",
              baseCurrency: "EUR",
              summaryStatus: "OK",
              cashTotalAmount: null,
              assetTotalAmount: null,
              totalAmount: "0.00000000",
              totalCurrency: "EUR",
            },
          ],
        });
      }

      if (query.includes("portfolio {")) {
        return gqlResponse({
          portfolio: {
            displayCurrency: "EUR",
            totalValueAmount: "0.00000000",
            accountTotals: [],
          },
        });
      }

      throw new Error(`Unhandled GQL query: ${query}`);
    });

    render(
      <ApolloProvider client={createTestClient()}>
        <UiStateProvider>
          <MemoryRouter initialEntries={["/accounts/new"]}>
            <Routes>
              <Route path="/accounts/new" element={<AccountNewPage />} />
              <Route path="/accounts" element={<AccountsListPage />} />
            </Routes>
          </MemoryRouter>
        </UiStateProvider>
      </ApolloProvider>,
    );

    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "New Broker" },
    });
    await screen.findByRole("option", { name: "CHF" });
    fireEvent.change(screen.getByLabelText("Base currency"), {
      target: { value: "EUR" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    const accountLink = await screen.findByRole("link", { name: /New Broker/ });
    expect(accountLink.getAttribute("href")).toBe("/accounts/99");
  });

  it("renders allowed currencies as dropdown options", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({ data: { currencies: ["CHF", "EUR", "GBP", "USD"] } }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    render(
      <ApolloProvider client={createTestClient()}>
        <MemoryRouter>
          <AccountNewPage />
        </MemoryRouter>
      </ApolloProvider>,
    );

    const baseCurrency = (await screen.findByLabelText(
      "Base currency",
    )) as HTMLSelectElement;

    expect(baseCurrency.tagName).toBe("SELECT");
    expect(
      Array.from(baseCurrency.options).map((option) => option.value),
    ).toEqual(["CHF", "EUR", "GBP", "USD"]);
  });
});
