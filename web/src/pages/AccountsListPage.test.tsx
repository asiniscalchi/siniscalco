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

import { UiStateProvider } from "@/lib/ui-state-provider";
import { AccountsListPage } from "./AccountsListPage";

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
  fxRates = {
    target_currency: "EUR",
    rates: [],
    last_updated: null,
    refresh_status: "available",
    refresh_error: null,
  },
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
  fxRates?: {
    target_currency: string;
    rates: { currency: string; rate: string }[];
    last_updated: string | null;
    refresh_status: "available" | "unavailable";
    refresh_error: string | null;
  };
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

    if (url.endsWith("/fx-rates")) {
      return Promise.resolve(
        new Response(JSON.stringify(fxRates), {
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
    expect(screen.getByText("Create account")).toBeTruthy();
    expect(
      document.querySelectorAll('[data-slot="card"]').length,
    ).toBeGreaterThan(0);
  });

  it("renders fetched account summaries and summary header", async () => {
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
      fxRates: {
        target_currency: "EUR",
        rates: [
          { currency: "USD", rate: "0.92" },
          { currency: "GBP", rate: "1.17" },
        ],
        last_updated: "2026-03-22 10:00:00",
        refresh_status: "available",
        refresh_error: null,
      },
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

    expect(await screen.findByText("Total Accounts")).toBeTruthy();
    expect(screen.getByText("Combined Balance")).toBeTruthy();
    expect(screen.getAllByText("1").length).toBeGreaterThan(0); // One for total accounts count

    expect(await screen.findByText("IBKR")).toBeTruthy();
    expect(screen.getByText(/broker/i)).toBeTruthy();
    expect(screen.getAllByText(/EUR/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("123.45 EUR").length).toBe(2);
    expect(screen.getByText("100.0%")).toBeTruthy(); // Weight

    expect(screen.getByText("USD")).toBeTruthy();
    expect(screen.getByText("0.9200")).toBeTruthy();
    expect(screen.getByText("GBP")).toBeTruthy();
    expect(screen.getByText("1.1700")).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: /IBKR.*broker.*EUR.*View details/i })
        .getAttribute("href"),
    ).toBe("/accounts/1");
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

    expect(await screen.findByText("Conversion unavailable")).toBeTruthy();
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

      if (url.endsWith("/fx-rates")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              target_currency: "EUR",
              rates: [],
              last_updated: null,
              refresh_status: "available",
              refresh_error: null,
            }),
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
    // fetch is called for /accounts, /fx-rates, /portfolio.
    // first attempt: /accounts (fail), /fx-rates (pass), /portfolio (pass) -> but they are parallel.
    // wait, if one fails, Promise.all rejects.
    // retry: calls all 3 again.
    // so total fetch calls should be 6.
    expect(fetch).toHaveBeenCalledTimes(6);
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
          name: /IBKR.*broker.*EUR.*View details/i,
        })
      ).getAttribute("href"),
    ).toBe("/accounts/7");
    expect(
      screen.getByRole("link", { name: "Create account" }).getAttribute("href"),
    ).toBe("/accounts/new");
  });

  it("renders sorted fx rates, excludes the identity rate, and keeps the card read-only", async () => {
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
      fxRates: {
        target_currency: "EUR",
        rates: [
          { currency: "CHF", rate: "1.04" },
          { currency: "GBP", rate: "1.17" },
          { currency: "USD", rate: "0.92" },
        ],
        last_updated: "2026-03-22 10:00:00",
        refresh_status: "available",
        refresh_error: null,
      },
    });

    renderAccountsListPage();

    const fxFooter = await screen.findByLabelText("FX rates against EUR");
    expect(fxFooter).toBeTruthy();

    const fxContent = within(fxFooter as HTMLElement);
    expect(fxContent.queryByText("CHF")).toBeTruthy();
    expect(fxContent.queryByText("1.0400")).toBeTruthy();
    expect(fxContent.queryByText("GBP")).toBeTruthy();
    expect(fxContent.queryByText("1.1700")).toBeTruthy();
    expect(fxContent.queryByText("USD")).toBeTruthy();
    expect(fxContent.queryByText("0.9200")).toBeTruthy();
    expect(fxContent.queryByText(/^EUR$/)).toBeNull();
    expect(fxContent.queryByRole("button")).toBeNull();
    expect(fxContent.queryByRole("textbox")).toBeNull();
  });

  it("renders the no-data state for fx rates", async () => {
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
      fxRates: {
        target_currency: "EUR",
        rates: [],
        last_updated: null,
        refresh_status: "available",
        refresh_error: null,
      },
    });

    renderAccountsListPage();

    // Footer is hidden when no rates are available
    expect(screen.queryByLabelText("FX rates against EUR")).toBeNull();
  });

  it("shows when fx refresh is unavailable while keeping the stored timestamp visible", async () => {
    mockDashboardRequests({
      accounts: [],
      fxRates: {
        target_currency: "EUR",
        rates: [{ currency: "USD", rate: "0.92" }],
        last_updated: "2026-03-22 10:00:00",
        refresh_status: "unavailable",
        refresh_error: "FX refresh unavailable: no successful refresh has completed",
      },
    });

    renderAccountsListPage();

    expect(await screen.findByText("Refresh Failed")).toBeTruthy();
    const errorIndicator = screen.getByText("Refresh Failed");
    expect(errorIndicator.getAttribute("title")).toContain("no successful refresh has completed");
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
      name: /IBKR.*broker.*EUR.*View details/i,
    });
    const maskedAmounts = screen.getAllByText("•••• EUR");

    expect(maskedAmounts.length).toBe(2);
    expect(screen.queryByText("123.45 EUR")).toBeNull();
    expect(accountLink.textContent).toContain("•••• EUR");
    expect(accountLink.textContent).not.toContain("123.45 EUR");
    expect(maskedAmounts[0].className).toContain("tabular-nums");
  });

});
