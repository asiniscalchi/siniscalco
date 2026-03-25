import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AccountNewPage } from ".";

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
        JSON.stringify([
          { code: "CHF" },
          { code: "EUR" },
          { code: "GBP" },
          { code: "USD" },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    render(
      <MemoryRouter>
        <AccountNewPage />
      </MemoryRouter>,
    );

    expect(screen.getByText("New Account")).toBeTruthy();
    expect(screen.getByLabelText("Name")).toBeTruthy();
    expect(screen.getByLabelText("Account type")).toBeTruthy();
    expect(screen.getByLabelText("Base currency")).toBeTruthy();
  });

  it("creates an account and returns to the accounts list route", async () => {
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

      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              id: 12,
              name: "IBKR",
              account_type: "broker",
              base_currency: "EUR",
              summary_status: "ok",
              total_amount: "0.00000000",
              total_currency: "EUR",
            }),
            { status: 201, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    render(
      <MemoryRouter initialEntries={["/accounts/new"]}>
        <Routes>
          <Route path="/accounts/new" element={<AccountNewPage />} />
          <Route path="/accounts" element={<div>Accounts Route</div>} />
        </Routes>
      </MemoryRouter>,
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
    expect(fetch).toHaveBeenCalledWith(
      expect.stringMatching(/\/accounts$/),
      expect.objectContaining({
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: "IBKR",
          account_type: "broker",
          base_currency: "EUR",
        }),
      }),
    );
  });

  it("creates a crypto account", async () => {
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

      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              id: 13,
              name: "Kraken",
              account_type: "crypto",
              base_currency: "EUR",
              summary_status: "ok",
              total_amount: "0.00000000",
              total_currency: "EUR",
            }),
            { status: 201, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    render(
      <MemoryRouter initialEntries={["/accounts/new"]}>
        <Routes>
          <Route path="/accounts/new" element={<AccountNewPage />} />
          <Route path="/accounts" element={<div>Accounts Route</div>} />
        </Routes>
      </MemoryRouter>,
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
    expect(fetch).toHaveBeenCalledWith(
      expect.stringMatching(/\/accounts$/),
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          name: "Kraken",
          account_type: "crypto",
          base_currency: "EUR",
        }),
      }),
    );
  });

  it("shows an API error when account creation fails", async () => {
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

      if (url.endsWith("/accounts")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              error: "validation_error",
              message: "currency must be one of: EUR, USD, GBP, CHF",
            }),
            { status: 400, headers: { "Content-Type": "application/json" } },
          ),
        );
      }

      throw new Error(`Unhandled fetch request: ${url}`);
    });

    render(
      <MemoryRouter>
        <AccountNewPage />
      </MemoryRouter>,
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

  it("renders allowed currencies as dropdown options", async () => {
    vi.mocked(fetch).mockResolvedValue(
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

    render(
      <MemoryRouter>
        <AccountNewPage />
      </MemoryRouter>,
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
