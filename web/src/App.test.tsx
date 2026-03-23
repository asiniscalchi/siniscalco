import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";

describe("App shell", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
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

    render(
      <MemoryRouter initialEntries={["/accounts"]}>
        <App />
      </MemoryRouter>,
    );

    expect(fetch).toHaveBeenCalledWith(expect.stringMatching(/\/health$/));
    expect(await screen.findByText("connected")).toBeTruthy();
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

    render(
      <MemoryRouter initialEntries={["/accounts"]}>
        <App />
      </MemoryRouter>,
    );

    expect(await screen.findByText("unavailable")).toBeTruthy();
  });

  it("renders the shell while page content and health are still loading", () => {
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}));

    render(
      <MemoryRouter initialEntries={["/accounts"]}>
        <App />
      </MemoryRouter>,
    );

    expect(screen.getByText("Siniscalco")).toBeTruthy();
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeTruthy();
    expect(screen.getByText("checking")).toBeTruthy();
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

    render(
      <MemoryRouter initialEntries={["/accounts"]}>
        <App />
      </MemoryRouter>,
    );

    const portfolioLink = screen.getByRole("link", { name: "Portfolio" });
    const accountsLink = screen.getByRole("link", { name: "Accounts" });
    expect(portfolioLink.getAttribute("aria-current")).toBeNull();
    expect(accountsLink.getAttribute("aria-current")).toBe("page");
    expect(accountsLink.className).toContain("border-foreground");

    expect(await screen.findAllByText("IBKR")).toHaveLength(2);
    expect(screen.getByText("connected")).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Create account" }));

    expect(await screen.findByText("New Account")).toBeTruthy();
    expect(screen.getByText("Siniscalco")).toBeTruthy();
    expect(screen.getByText("connected")).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: "Accounts" })
        .getAttribute("aria-current"),
    ).toBe("page");

    fireEvent.click(screen.getByRole("link", { name: "Cancel" }));

    expect(await screen.findAllByText("IBKR")).toHaveLength(2);

    fireEvent.click(
      screen.getByRole("link", { name: /IBKR.*broker.*EUR.*View details/ }),
    );

    expect(await screen.findByText("Account Summary")).toBeTruthy();
    expect(screen.getByText("Siniscalco")).toBeTruthy();
    expect(screen.getByText("connected")).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: "Accounts" })
        .getAttribute("aria-current"),
    ).toBe("page");
  });
});
