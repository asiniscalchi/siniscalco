import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UiStateProvider } from "@/lib/ui-state-provider";
import { AccountDetailPage } from ".";

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
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            error: "not_found",
            message: "Account not found",
          }),
          { status: 404, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            { code: "CHF" },
            { code: "EUR" },
            { code: "GBP" },
            { code: "USD" },
          ]),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
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
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
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

    renderAccountDetailPage("/accounts/8");

    expect(await screen.findByText("Could not load account")).toBeTruthy();
    expect(screen.getByText("Account not found")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText("Broker")).toBeTruthy();
    expect(screen.getByText("100.00000000")).toBeTruthy();
    expect(fetch).toHaveBeenCalledTimes(4);
  });

  it("upserts a balance from the account detail page", async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 9,
            name: "IBKR",
            account_type: "broker",
            base_currency: "EUR",
            created_at: "2026-03-22 00:00:00",
            balances: [],
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            { code: "CHF" },
            { code: "EUR" },
            { code: "GBP" },
            { code: "USD" },
          ]),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            currency: "USD",
            amount: "42.50000000",
            updated_at: "2026-03-22 00:00:00",
          }),
          { status: 201, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 9,
            name: "IBKR",
            account_type: "broker",
            base_currency: "EUR",
            created_at: "2026-03-22 00:00:00",
            balances: [
              {
                currency: "USD",
                amount: "42.50000000",
                updated_at: "2026-03-22 00:00:00",
              },
            ],
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
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
    expect(fetch).toHaveBeenNthCalledWith(
      3,
      expect.stringMatching(/\/accounts\/9\/balances\/USD$/),
      expect.objectContaining({
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ amount: "42.5" }),
      }),
    );
  });

  it("deletes a balance from the account detail page", async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 10,
            name: "Main Bank",
            account_type: "bank",
            base_currency: "USD",
            created_at: "2026-03-22 00:00:00",
            balances: [
              {
                currency: "USD",
                amount: "100.00000000",
                updated_at: "2026-03-22 00:00:00",
              },
            ],
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            { code: "CHF" },
            { code: "EUR" },
            { code: "GBP" },
            { code: "USD" },
          ]),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 10,
            name: "Main Bank",
            account_type: "bank",
            base_currency: "USD",
            created_at: "2026-03-22 00:00:00",
            balances: [],
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
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

    renderAccountDetailPage("/accounts/10");

    expect(await screen.findByText("100.00000000")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    expect(await screen.findByText("No balances yet")).toBeTruthy();
    expect(fetch).toHaveBeenNthCalledWith(
      3,
      expect.stringMatching(/\/accounts\/10\/balances\/USD$/),
      expect.objectContaining({
        method: "DELETE",
      }),
    );
  });

  it("deletes an account from the account detail page", async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 13,
            name: "Broker Account",
            account_type: "broker",
            base_currency: "EUR",
            created_at: "2026-03-22 00:00:00",
            balances: [],
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            { code: "CHF" },
            { code: "EUR" },
            { code: "GBP" },
            { code: "USD" },
          ]),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(new Response(null, { status: 204 }));

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
    expect(fetch).toHaveBeenNthCalledWith(
      3,
      expect.stringMatching(/\/accounts\/13$/),
      expect.objectContaining({
        method: "DELETE",
      }),
    );
  });

  it("renders an error when account deletion fails", async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 14,
            name: "Checking",
            account_type: "bank",
            base_currency: "USD",
            created_at: "2026-03-22 00:00:00",
            balances: [],
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            { code: "CHF" },
            { code: "EUR" },
            { code: "GBP" },
            { code: "USD" },
          ]),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            error: "conflict",
            message: "Could not delete account.",
          }),
          { status: 409, headers: { "Content-Type": "application/json" } },
        ),
      );

    renderAccountDetailPage("/accounts/14");

    expect(await screen.findByText("Checking")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Delete account" }));

    expect(await screen.findByText("Could not delete account.")).toBeTruthy();
  });

  it("resets the balance form when the loaded account changes", async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 11,
            name: "First Account",
            account_type: "broker",
            base_currency: "EUR",
            created_at: "2026-03-22 00:00:00",
            balances: [],
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            { code: "CHF" },
            { code: "EUR" },
            { code: "GBP" },
            { code: "USD" },
          ]),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 12,
            name: "Second Account",
            account_type: "bank",
            base_currency: "USD",
            created_at: "2026-03-22 00:00:00",
            balances: [],
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
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
