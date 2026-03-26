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

import { AssetsPage } from ".";

function renderAssetsPage() {
  return render(
    <MemoryRouter>
      <AssetsPage />
    </MemoryRouter>,
  );
}

async function unlockEditMode() {
  const unlockButton = await screen.findByRole("button", {
    name: /unlock edit mode/i,
  });
  fireEvent.click(unlockButton);
}

describe("AssetsPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
    vi.stubGlobal("confirm", vi.fn(() => true));
    vi.stubGlobal("alert", vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows a loading state before assets resolve", () => {
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}));

    renderAssetsPage();

    expect(screen.getByText("Assets")).toBeTruthy();
    expect(screen.getByText("Manage the assets you use in transactions.")).toBeTruthy();
  });

  it("renders fetched assets", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify([
          {
            id: 1,
            symbol: "AAPL",
            name: "Apple Inc.",
            asset_type: "STOCK",
            quote_symbol: "AAPL",
            isin: "US0378331005",
            current_price: "189.32",
            current_price_currency: "USD",
            current_price_as_of: "2026-03-24T14:30:00Z",
            total_quantity: "10.5",
          },
          {
            id: 2,
            symbol: "BTC",
            name: "Bitcoin",
            asset_type: "CRYPTO",
            quote_symbol: "BTC/USD",
            isin: null,
            current_price: null,
            current_price_currency: null,
            current_price_as_of: null,
            total_quantity: null,
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    expect((await screen.findAllByText("AAPL")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("Apple Inc.").length).toBeGreaterThan(0);
    expect(screen.getAllByText("STOCK").length).toBeGreaterThan(0);
    expect(screen.getAllByText("US0378331005").length).toBeGreaterThan(0);
    expect(screen.getAllByText("189.32 USD").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Pending").length).toBeGreaterThan(0);

    expect(screen.getAllByText("BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Bitcoin").length).toBeGreaterThan(0);
    expect(screen.getAllByText("CRYPTO").length).toBeGreaterThan(0);
    expect(screen.getAllByText("BTC/USD").length).toBeGreaterThan(0);
    expect(screen.getAllByText("1,987.86 USD").length).toBeGreaterThan(0);

    // Check that Actions column is NOT present when locked
    expect(screen.queryByText("Actions")).toBeNull();
  });

  it("shows empty state when no assets exist", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(JSON.stringify([]), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    renderAssetsPage();

    expect(await screen.findByText("No assets yet")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Add Asset" }).length).toBe(2);
  });

  it("keeps populated assets constrained on mobile when edit mode is unlocked", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify([
          {
            id: 1,
            symbol: "AAPL",
            name: "Apple Inc.",
            asset_type: "STOCK",
            quote_symbol: null,
            isin: "US0378331005",
            current_price: null,
            current_price_currency: null,
            current_price_as_of: null,
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    await screen.findAllByText("AAPL");
    await unlockEditMode();

    const pageRoot = screen.getByRole("heading", { name: "Assets", level: 1 }).closest("header")?.parentElement;
    const assetsCard = screen
      .getAllByText("AAPL")[0]
      .closest('[data-slot="card"]');
    const assetsScroller = screen.getByRole("table").parentElement;

    expect(pageRoot).toBeTruthy();
    expect(pageRoot?.className).toContain("min-w-0");
    expect(pageRoot?.className).toContain("overflow-x-hidden");
    expect(assetsCard).toBeTruthy();
    expect(assetsCard?.className).toContain("min-w-0");
    expect(assetsScroller?.className).toContain("overflow-x-auto");
    expect(screen.getByText("Actions")).toBeTruthy();
  });

  it("handles create asset", async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify([]), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    renderAssetsPage();

    // Use the first Add Asset button (the one in the header)
    const createButton = await screen.findAllByRole("button", { name: "Add Asset" });
    fireEvent.click(createButton[0]);

    expect(screen.getByText("Add Asset", { selector: "h2" })).toBeTruthy();

    fireEvent.change(screen.getByLabelText(/Symbol \*/), { target: { value: "MSFT" } });
    fireEvent.change(screen.getByLabelText(/Name \*/), { target: { value: "Microsoft" } });
    fireEvent.change(screen.getByLabelText(/Asset Type \*/), { target: { value: "STOCK" } });
    fireEvent.change(screen.getByLabelText(/Quote Symbol/), {
      target: { value: "MSFT" },
    });
    fireEvent.change(screen.getByLabelText(/ISIN/), { target: { value: "US5949181045" } });

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          id: 3,
          symbol: "MSFT",
          name: "Microsoft",
          asset_type: "STOCK",
          quote_symbol: "MSFT",
          isin: "US5949181045",
          current_price: null,
          current_price_currency: null,
          current_price_as_of: null,
        }),
        { status: 201, headers: { "Content-Type": "application/json" } },
      ),
    );

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify([
          {
            id: 3,
            symbol: "MSFT",
            name: "Microsoft",
            asset_type: "STOCK",
            quote_symbol: "MSFT",
            isin: "US5949181045",
            current_price: null,
            current_price_currency: null,
            current_price_as_of: null,
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    const modal = screen.getByRole("dialog");
    fireEvent.click(within(modal).getByRole("button", { name: "Add Asset" }));

    expect(fetch).toHaveBeenNthCalledWith(
      2,
      expect.any(String),
      expect.objectContaining({
        body: JSON.stringify({
          symbol: "MSFT",
          name: "Microsoft",
          asset_type: "STOCK",
          quote_symbol: "MSFT",
          isin: "US5949181045",
        }),
      }),
    );

    await waitFor(() => {
      expect(screen.queryByText("Add Asset", { selector: "h2" })).toBeNull();
    });

    expect((await screen.findAllByText("MSFT")).length).toBeGreaterThan(0);
  });

  it("handles edit asset", async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify([
          {
            id: 1,
            symbol: "AAPL",
            name: "Apple Inc.",
            asset_type: "STOCK",
            quote_symbol: "AAPL",
            isin: "US0378331005",
            current_price: null,
            current_price_currency: null,
            current_price_as_of: null,
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();
    await unlockEditMode();

    const editButton = (await screen.findAllByTitle("Edit asset"))[0];
    fireEvent.click(editButton);

    expect(screen.getByText("Edit Asset", { selector: "h2" })).toBeTruthy();
    expect((screen.getByLabelText(/Symbol \*/) as HTMLInputElement).value).toBe("AAPL");
    expect((screen.getByLabelText(/Quote Symbol/) as HTMLInputElement).value).toBe("AAPL");

    fireEvent.change(screen.getByLabelText(/Name \*/), { target: { value: "Apple Updated" } });

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          id: 1,
          symbol: "AAPL",
          name: "Apple Updated",
          asset_type: "STOCK",
          quote_symbol: "AAPL",
          isin: "US0378331005",
          current_price: null,
          current_price_currency: null,
          current_price_as_of: null,
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify([
          {
            id: 1,
            symbol: "AAPL",
            name: "Apple Updated",
            asset_type: "STOCK",
            quote_symbol: "AAPL",
            isin: "US0378331005",
            current_price: null,
            current_price_currency: null,
            current_price_as_of: null,
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    fireEvent.click(screen.getByRole("button", { name: "Save Changes" }));

    await waitFor(() => {
      expect(screen.queryByText("Edit Asset", { selector: "h2" })).toBeNull();
    });

    expect((await screen.findAllByText("Apple Updated")).length).toBeGreaterThan(0);
  });

  it("handles delete asset", async () => {
    let callCount = 0;
    vi.mocked(fetch).mockImplementation((url) => {
      callCount++;
      const urlStr = String(url);
      if (urlStr.endsWith("/assets") && callCount === 1) {
        return Promise.resolve(new Response(
          JSON.stringify([
            {
              id: 1,
              symbol: "AAPL",
              name: "Apple Inc.",
              asset_type: "STOCK",
              isin: "US0378331005",
            },
          ]),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ));
      }
      if (urlStr.includes("/assets/1") && callCount === 2) {
        return Promise.resolve(new Response(null, { status: 204 }));
      }
      if (urlStr.endsWith("/assets") && callCount === 3) {
        return Promise.resolve(new Response(JSON.stringify([]), { status: 200, headers: { "Content-Type": "application/json" } }));
      }
      return Promise.reject(new Error(`Unexpected fetch: ${urlStr} (call ${callCount})`));
    });

    renderAssetsPage();
    await unlockEditMode();

    const deleteButton = (await screen.findAllByTitle("Delete asset"))[0];
    fireEvent.click(deleteButton);

    expect(window.confirm).toHaveBeenCalledWith("Are you sure you want to delete AAPL?");

    await waitFor(() => {
      expect(screen.queryByText("AAPL")).toBeNull();
    }, { timeout: 2000 });

    expect(await screen.findByText("No assets yet")).toBeTruthy();
  });

  it("shows validation errors from backend", async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify([]), { status: 200, headers: { "Content-Type": "application/json" } }),
    );

    renderAssetsPage();

    const createButton = await screen.findAllByRole("button", { name: "Add Asset" });
    fireEvent.click(createButton[0]);

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          message: "Validation failed",
          field_errors: {
            symbol: ["Symbol is already taken"],
          },
        }),
        { status: 422, headers: { "Content-Type": "application/json" } },
      ),
    );

    fireEvent.change(screen.getByLabelText(/Symbol \*/), { target: { value: "AAPL" } });
    fireEvent.change(screen.getByLabelText(/Name \*/), { target: { value: "Apple" } });
    const modal = screen.getByRole("dialog");
    fireEvent.click(within(modal).getByRole("button", { name: "Add Asset" }));

    expect(await screen.findByText("Symbol is already taken")).toBeTruthy();
    expect(screen.getByText("Validation failed")).toBeTruthy();
  });
});
