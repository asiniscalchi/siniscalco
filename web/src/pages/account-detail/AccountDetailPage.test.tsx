import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UiStateProvider } from "@/lib/ui-state-provider";
import { AccountDetailPage } from ".";

const currenciesResponse = [
  { code: "CHF" },
  { code: "EUR" },
  { code: "GBP" },
  { code: "USD" },
];

function jsonResponse(data: unknown, status = 200) {
  return Promise.resolve(
    new Response(JSON.stringify(data), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

function mockAccountDetailFetch(
  handler: (url: string, init?: RequestInit) => Promise<Response> | Response,
) {
  vi.mocked(fetch).mockImplementation((input, init) => {
    const url = String(input);

    if (url.endsWith("/currencies")) {
      return jsonResponse(currenciesResponse);
    }

    if (url.endsWith("/assets")) {
      return jsonResponse([]);
    }

    if (url.includes("/positions")) {
      return jsonResponse([]);
    }

    return Promise.resolve(handler(url, init));
  });
}

function renderAccountDetailPage(initialEntry: string, routes?: ReactNode) {
  return render(
    <UiStateProvider>
      <MemoryRouter initialEntries={[initialEntry]}>
        <Routes>
          {routes ?? (
            <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
          )}
        </Routes>
      </MemoryRouter>
    </UiStateProvider>,
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
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/currencies")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              { code: "CHF" },
              { code: "EUR" },
              { code: "GBP" },
              { code: "USD" },
            ]),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      if (url.endsWith("/accounts/7")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              id: 7,
              name: "IBKR",
              account_type: "broker",
              base_currency: "EUR",
              created_at: "2026-03-22 00:00:00",
              balances: [
                {
                  currency: "USD",
                  amount: "12.30000000",
                  updated_at: "2026-03-22 00:00:00",
                },
              ],
            }),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    renderAccountDetailPage("/accounts/7");

    expect(await screen.findByText("IBKR")).toBeTruthy();
    expect(screen.getByText("broker · base currency EUR")).toBeTruthy();
    expect(screen.getByText("12.30000000")).toBeTruthy();
  });

  it("renders account assets when the account has positions", async () => {
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/accounts/7")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              id: 7,
              name: "IBKR",
              account_type: "broker",
              base_currency: "EUR",
              created_at: "2026-03-22 00:00:00",
              balances: [],
            }),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      if (url.endsWith("/currencies")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              { code: "CHF" },
              { code: "EUR" },
              { code: "GBP" },
              { code: "USD" },
            ]),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      if (url.endsWith("/accounts/7/positions")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              {
                account_id: 7,
                asset_id: 3,
                quantity: "2.500000",
              },
            ]),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      if (url.endsWith("/assets")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              {
                id: 3,
                symbol: "BTC",
                name: "Bitcoin",
                asset_type: "crypto",
                quote_symbol: "BTC-USD",
                isin: null,
                current_price: "90000.00",
                current_price_currency: "USD",
                current_price_as_of: "2026-03-22T00:00:00Z",
              },
            ]),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    renderAccountDetailPage("/accounts/7");

    expect(await screen.findByRole("heading", { name: "Assets" })).toBeTruthy();
    expect(screen.getByText("BTC")).toBeTruthy();
    expect(screen.getByText("Bitcoin")).toBeTruthy();
    expect(screen.getByText("2.500000")).toBeTruthy();
  });

  it("renders account detail with empty balances", async () => {
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/currencies")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              { code: "CHF" },
              { code: "EUR" },
              { code: "GBP" },
              { code: "USD" },
            ]),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      if (url.endsWith("/accounts/3")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              id: 3,
              name: "Main Bank",
              account_type: "bank",
              base_currency: "USD",
              created_at: "2026-03-22 00:00:00",
              balances: [],
            }),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    renderAccountDetailPage("/accounts/3");

    expect(await screen.findByText("Main Bank")).toBeTruthy();
    expect(screen.getByText("No balances yet")).toBeTruthy();
    expect(fetch).toHaveBeenCalledWith(expect.stringMatching(/\/accounts\/3$/));
  });

  it("renders an error state and retries the request", async () => {
    let accountRequestCount = 0;

    mockAccountDetailFetch((url) => {
      if (url.endsWith("/accounts/8")) {
        accountRequestCount += 1;

        if (accountRequestCount === 1) {
          return jsonResponse(
            {
              error: "not_found",
              message: "Account not found",
            },
            404,
          );
        }

        return jsonResponse({
          id: 8,
          name: "Broker",
          account_type: "broker",
          base_currency: "EUR",
          created_at: "2026-03-22 00:00:00",
          balances: [
            {
              currency: "EUR",
              amount: "100.00000000",
              updated_at: "2026-03-22 00:00:00",
            },
          ],
        });
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    renderAccountDetailPage("/accounts/8");

    expect(await screen.findByText("Could not load account")).toBeTruthy();
    expect(screen.getByText("Account not found")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText("Broker")).toBeTruthy();
    expect(screen.getByText("100.00000000")).toBeTruthy();
    expect(fetch).toHaveBeenCalledTimes(6);
  });

  it("upserts a balance from the account detail page", async () => {
    let saved = false;

    mockAccountDetailFetch((url, init) => {
      if (url.endsWith("/accounts/9")) {
        return jsonResponse({
          id: 9,
          name: "IBKR",
          account_type: "broker",
          base_currency: "EUR",
          created_at: "2026-03-22 00:00:00",
          balances: saved
            ? [
                {
                  currency: "USD",
                  amount: "42.50000000",
                  updated_at: "2026-03-22 00:00:00",
                },
              ]
            : [],
        });
      }

      if (url.endsWith("/accounts/9/balances/USD")) {
        expect(init).toEqual(
          expect.objectContaining({
            method: "PUT",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ amount: "42.5" }),
          }),
        );
        saved = true;
        return jsonResponse(
          {
            currency: "USD",
            amount: "42.50000000",
            updated_at: "2026-03-22 00:00:00",
          },
          201,
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
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
    expect(fetch).toHaveBeenCalledWith(
      expect.stringMatching(/\/accounts\/9\/balances\/USD$/),
      expect.objectContaining({
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ amount: "42.5" }),
      }),
    );
  });

  it("deletes a balance from the account detail page", async () => {
    let deleted = false;

    mockAccountDetailFetch((url, init) => {
      if (url.endsWith("/accounts/10")) {
        return jsonResponse({
          id: 10,
          name: "Main Bank",
          account_type: "bank",
          base_currency: "USD",
          created_at: "2026-03-22 00:00:00",
          balances: deleted
            ? []
            : [
                {
                  currency: "USD",
                  amount: "100.00000000",
                  updated_at: "2026-03-22 00:00:00",
                },
              ],
        });
      }

      if (url.endsWith("/accounts/10/balances/USD")) {
        expect(init).toEqual(expect.objectContaining({ method: "DELETE" }));
        deleted = true;
        return Promise.resolve(new Response(null, { status: 204 }));
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    renderAccountDetailPage("/accounts/10");

    expect(await screen.findByText("100.00000000")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    expect(await screen.findByText("No balances yet")).toBeTruthy();
    expect(fetch).toHaveBeenCalledWith(
      expect.stringMatching(/\/accounts\/10\/balances\/USD$/),
      expect.objectContaining({
        method: "DELETE",
      }),
    );
  });

  it("deletes an account from the account detail page", async () => {
    mockAccountDetailFetch((url, init) => {
      if (url.endsWith("/accounts/13")) {
        if (init?.method === "DELETE") {
          return Promise.resolve(new Response(null, { status: 204 }));
        }

        return jsonResponse({
          id: 13,
          name: "Broker Account",
          account_type: "broker",
          base_currency: "EUR",
          created_at: "2026-03-22 00:00:00",
          balances: [],
        });
      }

      throw new Error(`Unhandled fetch request: ${url}`);
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
    expect(fetch).toHaveBeenCalledWith(
      expect.stringMatching(/\/accounts\/13$/),
      expect.objectContaining({
        method: "DELETE",
      }),
    );
  });

  it("renders an error when account deletion fails", async () => {
    mockAccountDetailFetch((url, init) => {
      if (url.endsWith("/accounts/14")) {
        if (init?.method === "DELETE") {
          return jsonResponse(
            {
              error: "conflict",
              message: "Could not delete account.",
            },
            409,
          );
        }

        return jsonResponse({
          id: 14,
          name: "Checking",
          account_type: "bank",
          base_currency: "USD",
          created_at: "2026-03-22 00:00:00",
          balances: [],
        });
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    renderAccountDetailPage("/accounts/14");

    expect(await screen.findByText("Checking")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Delete account" }));

    expect(await screen.findByText("Could not delete account.")).toBeTruthy();
  });

  it("resets the balance form when the loaded account changes", async () => {
    mockAccountDetailFetch((url) => {
      if (url.endsWith("/accounts/11")) {
        return jsonResponse({
          id: 11,
          name: "First Account",
          account_type: "broker",
          base_currency: "EUR",
          created_at: "2026-03-22 00:00:00",
          balances: [],
        });
      }

      if (url.endsWith("/accounts/12")) {
        return jsonResponse({
          id: 12,
          name: "Second Account",
          account_type: "bank",
          base_currency: "USD",
          created_at: "2026-03-22 00:00:00",
          balances: [],
        });
      }

      throw new Error(`Unhandled fetch request: ${url}`);
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

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/currencies")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              { code: "CHF" },
              { code: "EUR" },
              { code: "GBP" },
              { code: "USD" },
            ]),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      if (url.endsWith("/accounts/7")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              id: 7,
              name: "IBKR",
              account_type: "broker",
              base_currency: "EUR",
              created_at: "2026-03-22 00:00:00",
              balances: [
                {
                  currency: "USD",
                  amount: "12.30000000",
                  updated_at: "2026-03-22 00:00:00",
                },
              ],
            }),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    renderAccountDetailPage("/accounts/7");

    expect(await screen.findByText("IBKR")).toBeTruthy();
    expect(screen.getByText("••••")).toBeTruthy();
    expect(screen.queryByText("12.30000000")).toBeNull();
  });
});
