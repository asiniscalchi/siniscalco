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

import { AccountsListPage } from "./AccountsListPage";

function mockDashboardRequests({
  accounts,
  fxRates = {
    target_currency: "EUR",
    rates: [],
    last_updated: null,
    refresh_status: "available",
    refresh_error: null,
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

    throw new Error(`Unhandled fetch request: ${url}`);
  });
}

describe("AccountsListPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows a loading state before accounts resolve", () => {
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}));

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    expect(screen.getByText("Accounts")).toBeTruthy();
    expect(screen.getByText("Create account")).toBeTruthy();
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
    });

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    expect(await screen.findAllByText("IBKR")).toHaveLength(2);
    expect(screen.getByText(/broker/)).toBeTruthy();
    expect(screen.getAllByText(/EUR/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("123.45 EUR").length).toBeGreaterThan(0);
    expect(screen.getByText("FX Rates")).toBeTruthy();
    expect(screen.getByText("Against EUR")).toBeTruthy();
    expect(screen.getByText("USD")).toBeTruthy();
    expect(screen.getByText("0.9200")).toBeTruthy();
    expect(screen.getByText("Last updated: 2026-03-22 10:00")).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: /IBKR.*broker.*EUR.*View details/ })
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

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    expect(await screen.findByText("Conversion unavailable")).toBeTruthy();
  });

  it("renders the empty state when no accounts exist", async () => {
    mockDashboardRequests({ accounts: [] });

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

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

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    expect(await screen.findByText("Could not load accounts")).toBeTruthy();

    fireEvent.click(screen.getByText("Retry"));

    await waitFor(() => {
      expect(screen.getAllByText("Main Bank").length).toBeGreaterThan(0);
    });
    expect(fetch).toHaveBeenCalledTimes(4);
  });

  it("renders the 'Cash by account' chart with correctly converted and sorted amounts", async () => {
    mockDashboardRequests({
      accounts: [
        {
          id: 1,
          name: "EUR Account",
          account_type: "bank",
          base_currency: "EUR",
          summary_status: "ok",
          total_amount: "100.00",
          total_currency: "EUR",
        },
        {
          id: 2,
          name: "USD Account",
          account_type: "bank",
          base_currency: "USD",
          summary_status: "ok",
          total_amount: "200.00",
          total_currency: "USD",
        },
      ],
      fxRates: {
        target_currency: "EUR",
        rates: [{ currency: "USD", rate: "0.5" }], // 200 USD * 0.5 = 100 EUR
        last_updated: "2026-03-22 10:00:00",
      },
    });

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    expect(await screen.findByText("Cash by account")).toBeTruthy();
    const chartCard = screen.getByText("Cash by account").closest('[data-slot="card"]')!;
    const chartWithin = within(chartCard as HTMLElement);

    expect(chartWithin.getByText("Distribution across accounts in EUR")).toBeTruthy();

    // Total should be 100 (EUR) + 100 (converted USD) = 200 EUR
    expect(chartWithin.getByText("Total")).toBeTruthy();
    expect(chartWithin.getByText("200.00 EUR")).toBeTruthy();

    // Account list should show original amounts
    expect(screen.getAllByText("100.00 EUR").length).toBeGreaterThan(0);
    expect(screen.getByText("200.00 USD")).toBeTruthy();

    // Chart should show converted amounts (both 100.00 EUR)
    const chartLabels = chartWithin.getAllByText("100.00 EUR");
    expect(chartLabels.length).toBe(2);
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

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    expect(
      (
        await screen.findByRole("link", {
          name: /IBKR.*broker.*EUR.*View details/,
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

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    const fxHeading = await screen.findByText("FX Rates");
    const fxCard = fxHeading.closest('[data-slot="card"]');

    expect(fxCard).toBeTruthy();

    const fxContent = within(fxCard as HTMLElement);
    expect(fxContent.queryByText("Against EUR")).toBeTruthy();
    expect(fxContent.queryByText("1.0400")).toBeTruthy();
    expect(fxContent.queryByText("1.1700")).toBeTruthy();
    expect(fxContent.queryByText("0.9200")).toBeTruthy();
    expect(fxContent.queryByText(/^EUR$/)).toBeNull();
    expect(fxContent.queryByText(/No FX data available/)).toBeNull();
    expect(fxContent.queryByRole("button")).toBeNull();
    expect(fxContent.queryByRole("textbox")).toBeNull();

    const rateItems = screen.getAllByRole("listitem");
    expect(rateItems.map((item) => item.textContent)).toEqual([
      "CHF1.0400",
      "GBP1.1700",
      "USD0.9200",
    ]);
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

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    expect(await screen.findByText("No FX data available")).toBeTruthy();
    expect(screen.getByText("Last updated: -")).toBeTruthy();
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

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>,
    );

    expect(await screen.findByText("Last updated: 2026-03-22 10:00")).toBeTruthy();
    expect(
      screen.getByText(
        "FX refresh unavailable: FX refresh unavailable: no successful refresh has completed",
      ),
    ).toBeTruthy();
  });
});
