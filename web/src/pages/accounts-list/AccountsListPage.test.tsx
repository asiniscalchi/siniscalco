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
import { AccountsListPage } from ".";

function renderAccountsListPage() {
  return render(
    <UiStateProvider>
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>
    </UiStateProvider>,
  );
}

function mockDashboardRequests({
  accounts,
  portfolio = {
    display_currency: "EUR",
    total_value_status: "ok",
    total_value_amount: "100.00000000",
    account_totals: [],
    cash_by_currency: [],
    fx_last_updated: null,
    fx_refresh_status: "available",
    fx_refresh_error: null,
  },
}: {
  accounts: unknown[];
  portfolio?: {
    display_currency: string;
    total_value_status: string;
    total_value_amount: string | null;
    account_totals: unknown[];
    cash_by_currency: unknown[];
    fx_last_updated: string | null;
    fx_refresh_status: string;
    fx_refresh_error: string | null;
  };
}) {
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

    if (url.endsWith("/portfolio")) {
      return Promise.resolve(
        new Response(JSON.stringify(portfolio), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );
    }

    throw new Error(`Unhandled fetch request: ${url}`);
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
          account_type: "broker",
          base_currency: "EUR",
          summary_status: "ok",
          total_amount: "123.45000000",
          total_currency: "EUR",
        },
      ],
      portfolio: {
        display_currency: "EUR",
        total_value_status: "ok",
        total_value_amount: "123.45000000",
        account_totals: [
          {
            id: 1,
            name: "IBKR",
            account_type: "broker",
            summary_status: "ok",
            total_amount: "123.45000000",
            total_currency: "EUR",
          },
        ],
        cash_by_currency: [],
        fx_last_updated: "2026-03-22 10:00:00",
        fx_refresh_status: "available",
        fx_refresh_error: null,
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
          account_type: "crypto",
          base_currency: "EUR",
          summary_status: "ok",
          total_amount: "0.00000000",
          total_currency: "EUR",
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
          account_type: "broker",
          base_currency: "EUR",
          summary_status: "conversion_unavailable",
          total_amount: null,
          total_currency: null,
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
    let accountsAttempt = 0;

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/accounts")) {
        accountsAttempt += 1;

        if (accountsAttempt === 1) {
          return Promise.reject(new Error("network error"));
        }

        return Promise.resolve(
          new Response(
            JSON.stringify([
              {
                id: 1,
                name: "Main Bank",
                account_type: "bank",
                base_currency: "USD",
                summary_status: "ok",
                total_amount: "50.00000000",
                total_currency: "USD",
              },
            ]),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      if (url.endsWith("/portfolio")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              display_currency: "EUR",
              total_value_status: "ok",
              total_value_amount: "50.00",
              account_totals: [],
              cash_by_currency: [],
              fx_last_updated: null,
              fx_refresh_status: "available",
              fx_refresh_error: null,
            }),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    renderAccountsListPage();

    expect(await screen.findByText("Could not load accounts")).toBeTruthy();

    fireEvent.click(screen.getByText("Retry"));

    await waitFor(() => {
      expect(screen.getByText("Main Bank")).toBeTruthy();
    });
    // fetch is called for /accounts and /portfolio.
    // first attempt: /accounts (fail), /portfolio (pass) -> Promise.all rejects.
    // retry: calls both again.
    // so total fetch calls should be 4.
    expect(fetch).toHaveBeenCalledTimes(4);
  });

  it("links to account detail and account creation routes", async () => {
    mockDashboardRequests({
      accounts: [
        {
          id: 7,
          name: "IBKR",
          account_type: "broker",
          base_currency: "EUR",
          summary_status: "ok",
          total_amount: "1.00000000",
          total_currency: "EUR",
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
          account_type: "broker",
          base_currency: "EUR",
          summary_status: "ok",
          total_amount: "123.45000000",
          total_currency: "EUR",
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
