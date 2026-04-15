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

import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";

import { AssetsPage } from ".";

function createTestClient() {
  return new ApolloClient({ link: new HttpLink({ uri: "http://localhost/graphql" }), cache: new InMemoryCache() });
}

function renderAssetsPage() {
  return render(
    <ApolloProvider client={createTestClient()}>
      <MemoryRouter>
        <AssetsPage />
      </MemoryRouter>
    </ApolloProvider>,
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
                quoteSourceSymbol: "AAPL",
                quoteSourceProvider: "yahoo",
                quoteSourceLastSuccessAt: "2026-03-24T14:30:00Z",
                currentPrice: "189.326789",
                currentPriceCurrency: "USD",
                currentPriceAsOf: "2026-03-24T14:30:00Z",
                totalQuantity: "10.5",
                convertedTotalValue: "1840.000000",
                convertedTotalValueCurrency: "EUR",
                avgCostBasis: null,
                avgCostBasisCurrency: null,
                previousClose: null,
                previousCloseCurrency: null,
              },
              {
                id: 2,
                symbol: "BTC",
                name: "Bitcoin",
                assetType: "CRYPTO",
                quoteSymbol: "BTC/USD",
                isin: null,
                quoteSourceSymbol: null,
                quoteSourceProvider: null,
                quoteSourceLastSuccessAt: null,
                currentPrice: null,
                currentPriceCurrency: null,
                currentPriceAsOf: null,
                totalQuantity: null,
                convertedTotalValue: null,
                convertedTotalValueCurrency: null,
                avgCostBasis: null,
                avgCostBasisCurrency: null,
                previousClose: null,
                previousCloseCurrency: null,
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
    expect(screen.getAllByText("$189.33").length).toBeGreaterThan(0);
    expect(screen.getByText("Updated 2026-03-24")).toBeTruthy();
    expect(screen.getAllByText("AAPL via Yahoo").length).toBeGreaterThan(0);
    expect(screen.getByTestId("asset-price-health").textContent).toBe(
      "Prices: 1 priced · 1 pending · 1 detected source",
    );
    expect(screen.getAllByText("Pending").length).toBeGreaterThan(0);

    expect(screen.getAllByText("BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Bitcoin").length).toBeGreaterThan(0);
    expect(screen.getAllByText("CRYPTO").length).toBeGreaterThan(0);
    expect(screen.getAllByText("BTC/USD").length).toBeGreaterThan(0);
    expect(screen.getAllByText("€1,840.00").length).toBeGreaterThan(0);

    // Check that Actions column is NOT present when locked
    expect(screen.queryByText("Actions")).toBeNull();
  });

  it("shows detected quote source details in the edit modal", async () => {
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
                quoteSourceSymbol: "AAPL",
                quoteSourceProvider: "twelve_data",
                quoteSourceLastSuccessAt: "2026-03-24T14:30:00Z",
                currentPrice: "189.326789",
                currentPriceCurrency: "USD",
                currentPriceAsOf: "2026-03-24T14:30:00Z",
                totalQuantity: null,
                convertedTotalValue: null,
                convertedTotalValueCurrency: null,
                avgCostBasis: null,
                avgCostBasisCurrency: null,
                previousClose: null,
                previousCloseCurrency: null,
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
    fireEvent.click((await screen.findAllByTitle("Edit asset"))[0]);

    expect(screen.getByText("Detected quote source")).toBeTruthy();
    expect(screen.getByText("AAPL via Twelve Data · Detected 2026-03-24")).toBeTruthy();
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
                quoteSourceSymbol: null,
                quoteSourceProvider: null,
                quoteSourceLastSuccessAt: null,
                currentPrice: null,
                currentPriceCurrency: null,
                currentPriceAsOf: null,
                totalQuantity: null,
                avgCostBasis: null,
                avgCostBasisCurrency: null,
                previousClose: null,
                previousCloseCurrency: null,
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

  it("keeps the mobile asset summary values on the right side of the card", async () => {
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
                quoteSourceSymbol: null,
                quoteSourceProvider: null,
                quoteSourceLastSuccessAt: null,
                currentPrice: "150.00",
                currentPriceCurrency: "USD",
                currentPriceAsOf: null,
                totalQuantity: "2",
                convertedTotalValue: "1840.000000",
                convertedTotalValueCurrency: "EUR",
                avgCostBasis: "100.00",
                avgCostBasisCurrency: "USD",
                previousClose: null,
                previousCloseCurrency: null,
              },
            ],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    await screen.findAllByText("AAPL");

    const mobileCard = screen.getByTestId("mobile-asset-card-1");
    const sideColumn = screen.getByTestId("mobile-asset-side-1");
    const isin = screen.getByTestId("mobile-asset-isin-1");
    const totalValue = screen.getByTestId("mobile-asset-total-value-1");
    const gainStack = screen.getByTestId("mobile-asset-gain-1");
    const gainPct = screen.getByTestId("mobile-asset-gain-pct-1");

    expect(mobileCard.className).toContain("items-start");
    expect(sideColumn.className).toContain("items-end");
    expect(sideColumn.className).toContain("text-right");
    expect(sideColumn.textContent).toContain("STOCK");
    expect(isin.className).toContain("mt-0.5");
    expect(isin.textContent).toContain("US0378331005");
    expect(totalValue.className).toContain("mt-0.5");
    expect(totalValue.textContent).toBe("€1,840.00");
    expect(gainStack.className).toContain("mt-auto");
    expect(gainStack.textContent).toContain("Gain: +$100.00");
    expect(gainPct.textContent).toBe("+50.00%");
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
                avgCostBasis: null,
                avgCostBasisCurrency: null,
                previousClose: null,
                previousCloseCurrency: null,
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
              avgCostBasis: null,
              avgCostBasisCurrency: null,
              previousClose: null,
              previousCloseCurrency: null,
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
                avgCostBasis: null,
                avgCostBasisCurrency: null,
                previousClose: null,
                previousCloseCurrency: null,
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
        return gqlResponse({ deleteAsset: 1 });
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

function makeAsset(overrides: Partial<{
  id: number;
  symbol: string;
  name: string;
  currentPrice: string | null;
  currentPriceCurrency: string | null;
  previousClose: string | null;
  previousCloseCurrency: string | null;
  quoteSourceSymbol: string | null;
  quoteSourceProvider: string | null;
  quoteSourceLastSuccessAt: string | null;
}> = {}) {
  return {
    id: overrides.id ?? 1,
    symbol: overrides.symbol ?? "AAPL",
    name: overrides.name ?? "Apple Inc.",
    assetType: "STOCK",
    quoteSymbol: null,
    isin: null,
    quoteSourceSymbol: overrides.quoteSourceSymbol ?? null,
    quoteSourceProvider: overrides.quoteSourceProvider ?? null,
    quoteSourceLastSuccessAt: overrides.quoteSourceLastSuccessAt ?? null,
    currentPrice: overrides.currentPrice ?? null,
    currentPriceCurrency: overrides.currentPriceCurrency ?? null,
    currentPriceAsOf: null,
    totalQuantity: null,
    avgCostBasis: null,
    avgCostBasisCurrency: null,
    previousClose: overrides.previousClose ?? null,
    previousCloseCurrency: overrides.previousCloseCurrency ?? null,
  };
}

describe("TopMoversCard", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("is hidden when no assets have previousClose data", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          data: {
            assets: [
              makeAsset({ id: 1, symbol: "AAPL", currentPrice: "150.00", currentPriceCurrency: "USD" }),
            ],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    await screen.findAllByText("AAPL");
    expect(screen.queryByText("Top Movers")).toBeNull();
  });

  it("shows winners and losers when daily gain data is present", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          data: {
            assets: [
              makeAsset({ id: 1, symbol: "WIN", name: "Winner Co", currentPrice: "105.00", currentPriceCurrency: "USD", previousClose: "100.00", previousCloseCurrency: "USD" }),
              makeAsset({ id: 2, symbol: "LOS", name: "Loser Co", currentPrice: "95.00", currentPriceCurrency: "USD", previousClose: "100.00", previousCloseCurrency: "USD" }),
            ],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    expect(await screen.findByText("Top Movers")).toBeTruthy();
    expect(screen.getByText("Winners")).toBeTruthy();
    expect(screen.getByText("Losers")).toBeTruthy();

    const winnersCol = screen.getByTestId("top-movers-winners");
    expect(winnersCol.textContent).toContain("WIN");
    expect(winnersCol.textContent).toContain("+5.00%");

    const losersCol = screen.getByTestId("top-movers-losers");
    expect(losersCol.textContent).toContain("LOS");
    expect(losersCol.textContent).toContain("-5.00%");
  });

  it("is hidden when all assets are perfectly flat", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          data: {
            assets: [
              makeAsset({ id: 1, symbol: "FLAT", currentPrice: "100.00", currentPriceCurrency: "USD", previousClose: "100.00", previousCloseCurrency: "USD" }),
            ],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    await screen.findAllByText("FLAT");
    expect(screen.queryByText("Top Movers")).toBeNull();
  });

  it("shows at most 3 winners and 3 losers", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          data: {
            assets: [
              makeAsset({ id: 1, symbol: "A1", currentPrice: "110.00", currentPriceCurrency: "USD", previousClose: "100.00", previousCloseCurrency: "USD" }),
              makeAsset({ id: 2, symbol: "A2", currentPrice: "108.00", currentPriceCurrency: "USD", previousClose: "100.00", previousCloseCurrency: "USD" }),
              makeAsset({ id: 3, symbol: "A3", currentPrice: "106.00", currentPriceCurrency: "USD", previousClose: "100.00", previousCloseCurrency: "USD" }),
              makeAsset({ id: 4, symbol: "A4", currentPrice: "104.00", currentPriceCurrency: "USD", previousClose: "100.00", previousCloseCurrency: "USD" }),
              makeAsset({ id: 5, symbol: "A5", currentPrice: "102.00", currentPriceCurrency: "USD", previousClose: "100.00", previousCloseCurrency: "USD" }),
            ],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    await screen.findByText("Top Movers");

    const winnersCol = screen.getByTestId("top-movers-winners");
    // Only A1, A2, A3 should appear (top 3); A4 and A5 should not
    expect(winnersCol.textContent).toContain("A1");
    expect(winnersCol.textContent).toContain("A2");
    expect(winnersCol.textContent).toContain("A3");
    expect(winnersCol.textContent).not.toContain("A4");
    expect(winnersCol.textContent).not.toContain("A5");
  });
});
