import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";
import { cleanup, render, screen } from "@testing-library/react";
import { type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UiStateProvider } from "@/lib/ui-state-provider";

import { PortfolioHistoryCard } from "./PortfolioHistoryCard";

vi.mock("recharts", () => ({
  Area: () => null,
  AreaChart: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  CartesianGrid: () => null,
  ResponsiveContainer: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  Tooltip: () => null,
  XAxis: ({
    dataKey,
    tickFormatter,
  }: {
    dataKey: string;
    tickFormatter?: (value: string) => string;
  }) => {
    const value = "2026-03-23";
    return (
      <div data-testid={`x-axis-${dataKey}`}>
        {tickFormatter ? tickFormatter(value) : value}
      </div>
    );
  },
  YAxis: () => null,
}));

function createTestClient() {
  return new ApolloClient({
    link: new HttpLink({ uri: "http://localhost/graphql" }),
    cache: new InMemoryCache(),
  });
}

function gqlResponse(data: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify({ data }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

function renderCard() {
  return render(
    <ApolloProvider client={createTestClient()}>
      <UiStateProvider>
        <PortfolioHistoryCard />
      </UiStateProvider>
    </ApolloProvider>,
  );
}

describe("PortfolioHistoryCard", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows history dates as yyyy-mm-dd", async () => {
    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body
        ? JSON.parse(String(init.body)) as { query: string }
        : null;
      const query = body?.query ?? "";

      if (query.includes("portfolioHistory")) {
        return gqlResponse({
          portfolioHistory: [
            {
              totalValue: "100.00000000",
              currency: "EUR",
              recordedAt: "2026-03-22 00:00:00",
            },
            {
              totalValue: "110.00000000",
              currency: "EUR",
              recordedAt: "2026-03-23 00:00:00",
            },
          ],
        });
      }

      return Promise.reject(new Error(`Unhandled GQL query: ${query}`));
    });

    renderCard();

    expect((await screen.findByTestId("x-axis-date")).textContent).toBe(
      "2026-03-23",
    );
    expect(screen.queryByText("Mar 23")).toBeNull();
  });
});
