import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";
import { useState } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { type PortfolioHistoryQuery } from "@/gql/types";
import { formatMoney } from "@/lib/format-money";
import { useUiState } from "@/lib/ui-state";

const PORTFOLIO_HISTORY_QUERY = gql`
  query PortfolioHistory {
    portfolioHistory {
      totalValue
      currency
      recordedAt
    }
  }
`;

type Range = "1W" | "1M" | "3M" | "6M" | "1Y" | "All";

const RANGES: Range[] = ["1W", "1M", "3M", "6M", "1Y", "All"];

const RANGE_DAYS: Record<Range, number | null> = {
  "1W": 7,
  "1M": 30,
  "3M": 90,
  "6M": 180,
  "1Y": 365,
  All: null,
};

type DataPoint = { date: string; value: number };

export function PortfolioHistoryCard({
  baseValue,
  currentValue,
}: {
  baseValue?: number;
  currentValue?: number;
}) {
  const { hideValues } = useUiState();
  const [range, setRange] = useState<Range>("1Y");
  const { data, loading } = useQuery<PortfolioHistoryQuery>(
    PORTFOLIO_HISTORY_QUERY,
    { fetchPolicy: "cache-and-network" },
  );

  const snapshots = data?.portfolioHistory ?? [];
  const currency = snapshots[0]?.currency;

  const rangeDays = RANGE_DAYS[range];
  const now = new Date();
  const cutoff =
    rangeDays != null
      ? new Date(now.getTime() - rangeDays * 24 * 60 * 60 * 1000)
      : null;

  const today = new Date().toISOString().slice(0, 10);
  const filtered: DataPoint[] = snapshots
    .filter((s) => !cutoff || new Date(s.recordedAt) >= cutoff)
    .map((s) => ({
      date: s.recordedAt.slice(0, 10),
      value: Number(s.totalValue),
    }));

  if (currentValue != null && (filtered.length === 0 || filtered[filtered.length - 1].date !== today)) {
    filtered.push({ date: today, value: currentValue });
  }

  const yDomain: [number, number | "auto"] = (() => {
    if (filtered.length < 2) return [0, "auto"];
    const domainValues = filtered.map((d) => d.value);

    if (baseValue != null && !Number.isNaN(baseValue)) {
      domainValues.push(baseValue);
    }

    const minValue = Math.min(...domainValues);
    const maxValue = Math.max(...domainValues);
    const padding = Math.max((maxValue - minValue) * 0.1, Math.abs(minValue) * 0.02);
    return [minValue - padding, maxValue + padding];
  })();
  const shouldShowBaseLine = baseValue != null && !Number.isNaN(baseValue);

  return (
    <Card className="bg-background">
      <CardHeader className="border-b">
        <div className="flex items-center justify-between">
          <CardTitle>Portfolio Value</CardTitle>
          <div className="flex gap-1">
            {RANGES.map((r) => (
              <button
                key={r}
                onClick={() => setRange(r)}
                className={`rounded px-2 py-1 text-xs font-medium transition-colors ${
                  range === r
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {r}
              </button>
            ))}
          </div>
        </div>
      </CardHeader>
      <CardContent className="pb-6 pt-6">
        {loading ? (
          <div className="flex h-48 items-center justify-center text-sm text-muted-foreground">
            Loading…
          </div>
        ) : filtered.length < 2 ? (
          <div className="flex h-48 items-center justify-center text-sm text-muted-foreground">
            Not enough data for the selected range.
          </div>
        ) : (
          <div className={hideValues ? "select-none blur-sm" : ""}>
            <ResponsiveContainer width="100%" height={240}>
              <AreaChart
                data={filtered}
                margin={{ top: 4, right: 4, left: 0, bottom: 0 }}
              >
                <defs>
                  <linearGradient
                    id="portfolioGradient"
                    x1="0"
                    y1="0"
                    x2="0"
                    y2="1"
                  >
                    <stop
                      offset="5%"
                      stopColor="#3b82f6"
                      stopOpacity={0.2}
                    />
                    <stop
                      offset="95%"
                      stopColor="#3b82f6"
                      stopOpacity={0}
                    />
                  </linearGradient>
                </defs>
                <CartesianGrid
                  strokeDasharray="3 3"
                  stroke="var(--border)"
                  vertical={false}
                />
                <XAxis
                  dataKey="date"
                  tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
                  tickLine={false}
                  axisLine={false}
                  minTickGap={40}
                />
                <YAxis
                  tickFormatter={formatCompact}
                  tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
                  tickMargin={4}
                  tickLine={false}
                  axisLine={false}
                  width={36}
                  domain={yDomain}
                />
                <Tooltip
                  content={
                    <ChartTooltip currency={currency} />
                  }
                />
                {shouldShowBaseLine ? (
                  <ReferenceLine
                    y={baseValue}
                    stroke="var(--muted-foreground)"
                    strokeDasharray="4 4"
                    label={{
                      value: "Base price",
                      fill: "var(--muted-foreground)",
                      fontSize: 11,
                      position: "insideTopRight",
                    }}
                  />
                ) : null}
                <Area
                  type="monotone"
                  dataKey="value"
                  stroke="#3b82f6"
                  strokeWidth={2}
                  fill="url(#portfolioGradient)"
                  dot={false}
                  activeDot={{ r: 4 }}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ChartTooltip({
  active,
  currency,
  ...rest
}: {
  active?: boolean;
  payload?: Array<{ value?: number }>;
  label?: string;
  currency?: string;
}) {
  const { payload, label } = rest as {
    payload?: Array<{ value?: number }>;
    label?: string;
  };
  if (!active || !payload?.length) return null;
  const value = payload[0]?.value;
  if (value == null) return null;
  return (
    <div className="rounded border border-border bg-background px-3 py-2 text-sm shadow-sm">
      <p className="text-muted-foreground">{label}</p>
      <p className="font-mono font-medium tabular-nums">
        {formatMoney(value, currency, false).text}
      </p>
    </div>
  );
}

function formatCompact(value: number): string {
  if (Math.abs(value) >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }
  if (Math.abs(value) >= 1_000) {
    return `${(value / 1_000).toFixed(0)}K`;
  }
  return String(value);
}
