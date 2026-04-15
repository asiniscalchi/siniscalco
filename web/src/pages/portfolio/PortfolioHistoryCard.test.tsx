import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";
import { cleanup, render, screen } from "@testing-library/react";
import { type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UiStateProvider } from "@/lib/ui-state-provider";

import { PortfolioHistoryCard } from "./PortfolioHistoryCard";

vi.mock("recharts", () => ({
  Area: () => null,
  AreaChart: ({
    children,
    margin,
  }: {
    children: ReactNode;
    margin?: { left?: number };
  }) => (
    <div data-left-margin={margin?.left} data-testid="portfolio-area-chart">
      {children}
    </div>
  ),
  CartesianGrid: () => null,
  ReferenceLine: ({
    label,
    stroke,
    strokeDasharray,
    y,
  }: {
    label?: { value?: string };
    stroke?: string;
    strokeDasharray?: string;
    y?: number;
  }) => (
    <div
      data-testid="portfolio-base-line"
      data-stroke={stroke}
      data-stroke-dasharray={strokeDasharray}
      data-y={y}
    >
      {label?.value}
    </div>
  ),
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
  YAxis: ({
    domain,
    tickMargin,
    width,
  }: {
    domain?: [number, number | "auto"];
    tickMargin?: number;
    width?: number;
  }) => (
    <div
      data-domain={JSON.stringify(domain)}
      data-testid="portfolio-y-axis"
      data-tick-margin={tickMargin}
      data-width={width}
    />
  ),
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

function renderCard(props?: { baseValue?: number; currentValue?: number }) {
  return render(
    <ApolloProvider client={createTestClient()}>
      <UiStateProvider>
        <PortfolioHistoryCard {...props} />
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

  it("keeps the chart left gutter compact", async () => {
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

    expect(
      (await screen.findByTestId("portfolio-area-chart")).getAttribute(
        "data-left-margin",
      ),
    ).toBe("0");
    expect(screen.getByTestId("portfolio-y-axis").getAttribute("data-width")).toBe(
      "36",
    );
    expect(
      screen.getByTestId("portfolio-y-axis").getAttribute("data-tick-margin"),
    ).toBe("4");
  });

  it("shows the base price line and includes it in the y-axis domain", async () => {
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

    renderCard({ baseValue: 80 });

    const baseLine = await screen.findByTestId("portfolio-base-line");
    expect(baseLine.textContent).toBe("Base price");
    expect(baseLine.getAttribute("data-y")).toBe("80");
    expect(baseLine.getAttribute("data-stroke")).toBe("var(--muted-foreground)");
    expect(baseLine.getAttribute("data-stroke-dasharray")).toBe("4 4");

    const domain = JSON.parse(
      screen.getByTestId("portfolio-y-axis").getAttribute("data-domain") ?? "[]",
    ) as [number, number];
    expect(domain[0]).toBeLessThan(80);
  });
});
