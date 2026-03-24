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
            isin: "US0378331005",
          },
          {
            id: 2,
            symbol: "BTC",
            name: "Bitcoin",
            asset_type: "CRYPTO",
            isin: null,
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    expect(await screen.findByText("AAPL")).toBeTruthy();
    expect(screen.getByText("Apple Inc.")).toBeTruthy();
    expect(screen.getByText("STOCK")).toBeTruthy();
    expect(screen.getByText("US0378331005")).toBeTruthy();

    expect(screen.getByText("BTC")).toBeTruthy();
    expect(screen.getByText("Bitcoin")).toBeTruthy();
    expect(screen.getByText("CRYPTO")).toBeTruthy();

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
            isin: "US0378331005",
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();

    await screen.findByText("AAPL");
    await unlockEditMode();

    const pageRoot = screen.getByText("Assets").closest("header")?.parentElement;
    const assetsCard = screen
      .getByText("AAPL")
      .closest('[data-slot="card"]');
    const assetsScroller = screen.getByRole("table").parentElement;

    expect(pageRoot).toBeTruthy();
    expect(pageRoot?.className).toContain("min-w-0");
    expect(assetsCard).toBeTruthy();
    expect(assetsCard?.className).toContain("min-w-0");
    expect(assetsScroller?.className).toContain("overflow-x-auto");
    expect(assetsScroller?.className).toContain("overflow-y-hidden");
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
    fireEvent.change(screen.getByLabelText(/ISIN/), { target: { value: "US5949181045" } });

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          id: 3,
          symbol: "MSFT",
          name: "Microsoft",
          asset_type: "STOCK",
          isin: "US5949181045",
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
            isin: "US5949181045",
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    const modal = screen.getByRole("dialog");
    fireEvent.click(within(modal).getByRole("button", { name: "Add Asset" }));

    await waitFor(() => {
      expect(screen.queryByText("Add Asset", { selector: "h2" })).toBeNull();
    });

    expect(await screen.findByText("MSFT")).toBeTruthy();
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
            isin: "US0378331005",
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    renderAssetsPage();
    await unlockEditMode();

    const editButton = await screen.findByTitle("Edit asset");
    fireEvent.click(editButton);

    expect(screen.getByText("Edit Asset", { selector: "h2" })).toBeTruthy();
    expect((screen.getByLabelText(/Symbol \*/) as HTMLInputElement).value).toBe("AAPL");

    fireEvent.change(screen.getByLabelText(/Name \*/), { target: { value: "Apple Updated" } });

    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          id: 1,
          symbol: "AAPL",
          name: "Apple Updated",
          asset_type: "STOCK",
          isin: "US0378331005",
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
            isin: "US0378331005",
          },
        ]),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    fireEvent.click(screen.getByRole("button", { name: "Save Changes" }));

    await waitFor(() => {
      expect(screen.queryByText("Edit Asset", { selector: "h2" })).toBeNull();
    });

    expect(await screen.findByText("Apple Updated")).toBeTruthy();
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

    const deleteButton = await screen.findByTitle("Delete asset");
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
