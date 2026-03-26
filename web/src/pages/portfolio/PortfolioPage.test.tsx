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

import { PortfolioPage } from ".";

function renderPortfolioPage() {
  return render(
    <UiStateProvider>
      <MemoryRouter>
        <PortfolioPage />
      </MemoryRouter>
    </UiStateProvider>,
  );
}

const defaultFxRates = {
  target_currency: "EUR",
  rates: [],
  last_updated: null,
  refresh_status: "available",
  refresh_error: null,
};

function mockPortfolioRequest(summary: unknown) {
  vi.mocked(fetch).mockImplementation((input) => {
    const url = String(input);

    if (url.endsWith("/portfolio")) {
      return Promise.resolve(
        new Response(JSON.stringify(summary), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );
    }

    if (url.endsWith("/fx-rates")) {
      return Promise.resolve(
        new Response(JSON.stringify(defaultFxRates), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );
    }

    throw new Error(`Unhandled fetch request: ${url}`);
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
      display_currency: "EUR",
      total_value_status: "ok",
      total_value_amount: "153.70000000",
      account_totals: [
        {
          id: 1,
          name: "IBKR",
          account_type: "broker",
          summary_status: "ok",
          total_amount: "103.70000000",
          total_currency: "EUR",
        },
        {
          id: 2,
          name: "Main Bank",
          account_type: "bank",
          summary_status: "ok",
          total_amount: "50.00000000",
          total_currency: "EUR",
        },
      ],
      cash_by_currency: [
        { currency: "EUR", amount: "50.00000000", converted_amount: "50.00000000" },
        { currency: "GBP", amount: "10.00000000", converted_amount: "11.70000000" },
        { currency: "USD", amount: "100.00000000", converted_amount: "92.00000000" },
      ],
      fx_last_updated: "2026-03-22 11:30:00",
      fx_refresh_status: "available",
      fx_refresh_error: null,
      allocation_totals: [{ label: "Cash", amount: "153.70000000" }],
      allocation_is_partial: false,
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
      display_currency: "EUR",
      total_value_status: "ok",
      total_value_amount: "0.00000000",
      account_totals: [
        {
          id: 1,
          name: "IBKR",
          account_type: "broker",
          summary_status: "ok",
          total_amount: "0.00000000",
          total_currency: "EUR",
        },
      ],
      cash_by_currency: [],
      fx_last_updated: null,
      fx_refresh_status: "unavailable",
      fx_refresh_error: "FX refresh unavailable: no successful refresh has completed",
      allocation_totals: [],
      allocation_is_partial: false,
    });

    renderPortfolioPage();

    expect(await screen.findByText("No portfolio cash data yet")).toBeTruthy();
  });

  it("renders conversion unavailable while keeping original cash balances visible", async () => {
    mockPortfolioRequest({
      display_currency: "EUR",
      total_value_status: "conversion_unavailable",
      total_value_amount: null,
      account_totals: [
        {
          id: 1,
          name: "IBKR",
          account_type: "broker",
          summary_status: "conversion_unavailable",
          total_amount: null,
          total_currency: "EUR",
        },
      ],
      cash_by_currency: [
        { currency: "GBP", amount: "10.00000000", converted_amount: null },
        { currency: "USD", amount: "100.00000000", converted_amount: null },
      ],
      fx_last_updated: "2026-03-22 10:00:00",
      fx_refresh_status: "unavailable",
      fx_refresh_error: "FX refresh unavailable: provider returned status 500",
      allocation_totals: [],
      allocation_is_partial: true,
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

    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/portfolio")) {
        attempt += 1;

        if (attempt === 1) {
          return Promise.reject(new Error("network error"));
        }

        return Promise.resolve(
          new Response(
            JSON.stringify({
              display_currency: "EUR",
              total_value_status: "ok",
              total_value_amount: "1.00000000",
              account_totals: [],
              cash_by_currency: [{ currency: "EUR", amount: "1.00000000", converted_amount: "1.00000000" }],
              fx_last_updated: null,
              fx_refresh_status: "available",
              fx_refresh_error: null,
              allocation_totals: [{ label: "Cash", amount: "1.00000000" }],
              allocation_is_partial: false,
            }),
            { status: 200, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      if (url.endsWith("/fx-rates")) {
        return Promise.resolve(
          new Response(JSON.stringify(defaultFxRates), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
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
      display_currency: "EUR",
      total_value_status: "ok",
      total_value_amount: "153.70000000",
      account_totals: [
        {
          id: 1,
          name: "IBKR",
          account_type: "broker",
          summary_status: "ok",
          total_amount: "103.70000000",
          total_currency: "EUR",
        },
      ],
      cash_by_currency: [
        { currency: "USD", amount: "100.00000000", converted_amount: "92.00000000" },
      ],
      fx_last_updated: "2026-03-22 11:30:00",
      fx_refresh_status: "available",
      fx_refresh_error: null,
      allocation_totals: [{ label: "Cash", amount: "92.00000000" }],
      allocation_is_partial: false,
    });

    renderPortfolioPage();

    expect(await screen.findAllByText("•••• EUR")).toHaveLength(3);
  });

  it("handles missing currency conversion values without crashing", async () => {
    mockPortfolioRequest({
      display_currency: "EUR",
      total_value_status: "ok",
      total_value_amount: "10.00000000",
      account_totals: [
        {
          id: 1,
          name: "Empty Account",
          account_type: "bank",
          summary_status: "conversion_unavailable",
          total_amount: null,
          total_currency: "EUR",
        },
      ],
      cash_by_currency: [
        { currency: "JPY", amount: "1000.00000000", converted_amount: null },
      ],
      fx_last_updated: null,
      fx_refresh_status: "available",
      fx_refresh_error: null,
      allocation_totals: [],
      allocation_is_partial: true,
    });

    renderPortfolioPage();

    expect(await screen.findByText("Conversion unavailable")).toBeTruthy();
    expect(screen.queryByText("Conversion data unavailable")).toBeNull();
  });

  it("renders the allocation card with slices and labels", async () => {
    mockPortfolioRequest({
      display_currency: "EUR",
      total_value_status: "ok",
      total_value_amount: "300.00000000",
      account_totals: [],
      cash_by_currency: [
        { currency: "EUR", amount: "100.00000000", converted_amount: "100.00000000" },
      ],
      fx_last_updated: null,
      fx_refresh_status: "available",
      fx_refresh_error: null,
      allocation_totals: [
        { label: "Stock", amount: "200.00000000" },
        { label: "Cash", amount: "100.00000000" },
      ],
      allocation_is_partial: false,
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

  it("shows the partial banner when allocation_is_partial is true", async () => {
    mockPortfolioRequest({
      display_currency: "EUR",
      total_value_status: "ok",
      total_value_amount: "100.00000000",
      account_totals: [],
      cash_by_currency: [
        { currency: "EUR", amount: "100.00000000", converted_amount: "100.00000000" },
      ],
      fx_last_updated: null,
      fx_refresh_status: "available",
      fx_refresh_error: null,
      allocation_totals: [{ label: "Cash", amount: "100.00000000" }],
      allocation_is_partial: true,
    });

    renderPortfolioPage();

    expect(
      await screen.findByText(
        "Allocation incomplete: some assets could not be valued.",
      ),
    ).toBeTruthy();
  });

  it("shows no-data message when allocation_totals is empty and not partial", async () => {
    mockPortfolioRequest({
      display_currency: "EUR",
      total_value_status: "ok",
      total_value_amount: "100.00000000",
      account_totals: [],
      cash_by_currency: [
        { currency: "EUR", amount: "100.00000000", converted_amount: "100.00000000" },
      ],
      fx_last_updated: null,
      fx_refresh_status: "available",
      fx_refresh_error: null,
      allocation_totals: [],
      allocation_is_partial: false,
    });

    renderPortfolioPage();

    expect(await screen.findByText("No allocation data available.")).toBeTruthy();
  });

  it("masks allocation amounts and percentages in privacy mode", async () => {
    window.localStorage.setItem("ui.hide_values", "true");

    mockPortfolioRequest({
      display_currency: "EUR",
      total_value_status: "ok",
      total_value_amount: "300.00000000",
      account_totals: [],
      cash_by_currency: [
        { currency: "EUR", amount: "100.00000000", converted_amount: "100.00000000" },
      ],
      fx_last_updated: null,
      fx_refresh_status: "available",
      fx_refresh_error: null,
      allocation_totals: [
        { label: "Stock", amount: "200.00000000" },
        { label: "Cash", amount: "100.00000000" },
      ],
      allocation_is_partial: false,
    });

    renderPortfolioPage();

    await screen.findByText("Allocation by asset class");
    expect(screen.queryByText("200.00 EUR")).toBeNull();
    expect(screen.queryByText("66.7%")).toBeNull();
    expect(screen.getAllByText("•••%").length).toBeGreaterThan(0);
  });
});
