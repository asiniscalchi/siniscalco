import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UiStateProvider } from "@/lib/ui-state-provider";
import App from "./App";

function renderApp(initialEntries: string[]) {
  return render(
    <UiStateProvider>
      <MemoryRouter initialEntries={initialEntries}>
        <App />
      </MemoryRouter>
    </UiStateProvider>,
  );
}

describe("App shell", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("requests health on shell mount and shows connected from the response status", async () => {
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/health")) {
        return Promise.resolve(
          new Response("not-used", {
            status: 200,
            headers: { "Content-Type": "text/plain" },
          }),
        );
      }

      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(JSON.stringify([]), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
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
              total_value_amount: "0.00000000",
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

    renderApp(["/accounts"]);

    expect(fetch).toHaveBeenCalledWith(expect.stringMatching(/\/health$/));
    expect(await screen.findByTitle("Backend: connected")).toBeTruthy();
  });

  it("shows unavailable when the health request returns a non-success status", async () => {
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/health")) {
        return Promise.resolve(new Response("down", { status: 503 }));
      }

      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(JSON.stringify([]), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
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
              total_value_amount: "0.00000000",
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

    renderApp(["/accounts"]);

    expect(await screen.findByTitle("Backend: unavailable")).toBeTruthy();
  });

  it("renders the shell while page content and health are still loading", () => {
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}));

    renderApp(["/accounts"]);

    expect(screen.getByLabelText("Siniscalco")).toBeTruthy();
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeTruthy();
    expect(screen.getByTitle("Backend: checking")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Portfolio" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Accounts" })).toBeTruthy();
  });

  it("keeps the shell rendered while navigating between wrapped routes", async () => {
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/health")) {
        return Promise.resolve(new Response("ok", { status: 200 }));
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
              balances: [],
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
              total_value_amount: "1.00000000",
              account_totals: [
                {
                  id: 7,
                  name: "IBKR",
                  account_type: "broker",
                  summary_status: "ok",
                  total_amount: "1.00000000",
                  total_currency: "EUR",
                },
              ],
              cash_by_currency: [{ currency: "EUR", amount: "1.00000000" }],
              fx_last_updated: null,
              fx_refresh_status: "available",
              fx_refresh_error: null,
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

      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              {
                id: 7,
                name: "IBKR",
                account_type: "broker",
                base_currency: "EUR",
                summary_status: "ok",
                total_amount: "1.00000000",
                total_currency: "EUR",
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
      screen.getByRole("link", { name: /IBKR.*broker.*EUR.*View details/ }),
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
    vi.mocked(fetch).mockImplementation((input) => {
      const url = String(input);

      if (url.endsWith("/health")) {
        return Promise.resolve(new Response("ok", { status: 200 }));
      }

      if (url.endsWith("/portfolio")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
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
                {
                  currency: "USD",
                  amount: "100.00000000",
                  converted_amount: "92.00000000",
                },
              ],
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

    const view = renderApp(["/portfolio"]);

    const totalAmount = await screen.findByText("153.70 EUR");
    const initialWidth = totalAmount.getAttribute("style");
    expect(screen.getByText("103.70 EUR")).toBeTruthy();
    expect(screen.getByText("100 USD")).toBeTruthy();

    fireEvent.click(
      screen.getByRole("button", { name: "Hide financial values" }),
    );

    expect(await screen.findAllByText("•••• EUR")).toHaveLength(2);
    expect(screen.getByText("•••• USD")).toBeTruthy();
    expect(screen.queryByText("153.70 EUR")).toBeNull();
    expect(screen.queryByText("103.70 EUR")).toBeNull();
    expect(screen.queryByText("100 USD")).toBeNull();
    expect(window.localStorage.getItem("ui.hide_values")).toBe("true");
    expect(screen.getAllByText("•••• EUR")[0].getAttribute("style")).toBe(
      initialWidth,
    );

    view.unmount();
    renderApp(["/portfolio"]);

    expect(await screen.findAllByText("•••• EUR")).toHaveLength(2);
    expect(screen.getByRole("button", { name: "Show financial values" })).toBeTruthy();
    expect(screen.queryByText("153.70 EUR")).toBeNull();
  });
});
