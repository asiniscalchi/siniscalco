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

function gqlResponse(data: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify({ data }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

function gqlErrorResponse(message: string, fieldErrors?: Record<string, string[]>) {
  const errors: Array<{ message: string; extensions?: Record<string, unknown> }> = [
    fieldErrors
      ? { message, extensions: { field_errors: fieldErrors } }
      : { message },
  ];
  return Promise.resolve(
    new Response(
      JSON.stringify({ data: null, errors }),
      {
        status: 200,
        headers: { "Content-Type": "application/json" },
      },
    ),
  );
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

    expect(document.querySelector(".animate-pulse")).toBeTruthy();
  });

  it("renders fetched assets", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          data: {
            assets: [
              {
                id: 1,
                symbol: "AAPL",
                name: "Apple Inc.",
                assetType: "STOCK",
                quoteSymbol: "AAPL",
                isin: "US0378331005",
                currentPrice: "189.32",
                currentPriceCurrency: "USD",
                currentPriceAsOf: "2026-03-24T14:30:00Z",
                totalQuantity: "10.5",
              },
              {
                id: 2,
                symbol: "BTC",
                name: "Bitcoin",
                assetType: "CRYPTO",
                quoteSymbol: "BTC/USD",
                isin: null,
                currentPrice: null,
                currentPriceCurrency: null,
                currentPriceAsOf: null,
                totalQuantity: null,
              },
            ],
          },
        }),
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
      new Response(JSON.stringify({ data: { assets: [] } }), {
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
        JSON.stringify({
          data: {
            assets: [
              {
                id: 1,
                symbol: "AAPL",
                name: "Apple Inc.",
                assetType: "STOCK",
                quoteSymbol: null,
                isin: "US0378331005",
                currentPrice: null,
                currentPriceCurrency: null,
                currentPriceAsOf: null,
                totalQuantity: null,
              },
            ],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    await screen.findAllByText("AAPL");
    await unlockEditMode();

    const pageRoot = screen.getByRole("heading", { name: "Assets", level: 1 }).closest('[data-slot="card"]')?.parentElement;
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
      new Response(JSON.stringify({ data: { assets: [] } }), {
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
          data: {
            createAsset: {
              id: 3,
              symbol: "MSFT",
              name: "Microsoft",
              assetType: "STOCK",
              quoteSymbol: "MSFT",
              isin: "US5949181045",
              currentPrice: null,
              currentPriceCurrency: null,
              currentPriceAsOf: null,
              totalQuantity: null,
            },
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          data: {
            assets: [
              {
                id: 3,
                symbol: "MSFT",
                name: "Microsoft",
                assetType: "STOCK",
                quoteSymbol: "MSFT",
                isin: "US5949181045",
                currentPrice: null,
                currentPriceCurrency: null,
                currentPriceAsOf: null,
                totalQuantity: null,
              },
            ],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    const modal = screen.getByRole("dialog");
    fireEvent.click(within(modal).getByRole("button", { name: "Add Asset" }));

    await waitFor(() => {
      expect(screen.queryByText("Add Asset", { selector: "h2" })).toBeNull();
    });

    expect((await screen.findAllByText("MSFT")).length).toBeGreaterThan(0);
  });

  it("handles edit asset", async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          data: {
            assets: [
              {
                id: 1,
                symbol: "AAPL",
                name: "Apple Inc.",
                assetType: "STOCK",
                quoteSymbol: "AAPL",
                isin: "US0378331005",
                currentPrice: null,
                currentPriceCurrency: null,
                currentPriceAsOf: null,
                totalQuantity: null,
              },
            ],
          },
        }),
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
          data: {
            updateAsset: {
              id: 1,
              symbol: "AAPL",
              name: "Apple Updated",
              assetType: "STOCK",
              quoteSymbol: "AAPL",
              isin: "US0378331005",
              currentPrice: null,
              currentPriceCurrency: null,
              currentPriceAsOf: null,
              totalQuantity: null,
            },
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          data: {
            assets: [
              {
                id: 1,
                symbol: "AAPL",
                name: "Apple Updated",
                assetType: "STOCK",
                quoteSymbol: "AAPL",
                isin: "US0378331005",
                currentPrice: null,
                currentPriceCurrency: null,
                currentPriceAsOf: null,
                totalQuantity: null,
              },
            ],
          },
        }),
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
    vi.mocked(fetch).mockImplementation((_input, init) => {
      callCount++;
      const body = init?.body ? JSON.parse(String(init.body)) as { query: string } : null;
      const query = body?.query ?? "";

      if (query.includes("assets") && callCount === 1) {
        return gqlResponse({
          assets: [
            {
              id: 1,
              symbol: "AAPL",
              name: "Apple Inc.",
              assetType: "STOCK",
              isin: "US0378331005",
              quoteSymbol: null,
              currentPrice: null,
              currentPriceCurrency: null,
              currentPriceAsOf: null,
              totalQuantity: null,
            },
          ],
        });
      }
      if (query.includes("deleteAsset") && callCount === 2) {
        return gqlResponse({ deleteAsset: true });
      }
      if (query.includes("assets") && callCount === 3) {
        return gqlResponse({ assets: [] });
      }
      return Promise.reject(new Error(`Unexpected GQL query (call ${callCount}): ${query}`));
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
      new Response(JSON.stringify({ data: { assets: [] } }), { status: 200, headers: { "Content-Type": "application/json" } }),
    );

    renderAssetsPage();

    const createButton = await screen.findAllByRole("button", { name: "Add Asset" });
    fireEvent.click(createButton[0]);

    vi.mocked(fetch).mockReturnValueOnce(
      gqlErrorResponse("Validation failed", {
        symbol: ["Symbol is already taken"],
      }),
    );

    fireEvent.change(screen.getByLabelText(/Symbol \*/), { target: { value: "AAPL" } });
    fireEvent.change(screen.getByLabelText(/Name \*/), { target: { value: "Apple" } });
    const modal = screen.getByRole("dialog");
    fireEvent.click(within(modal).getByRole("button", { name: "Add Asset" }));

    expect(await screen.findByText("Symbol is already taken")).toBeTruthy();
    expect(screen.getByText("Validation failed")).toBeTruthy();
  });
});
