import { useEffect, useMemo, useState } from "react";
import { useApolloClient, useQuery } from "@apollo/client/react";
import { NavLink, Outlet } from "react-router-dom";

import { AssistantPanel } from "@/components/AssistantPanel";
import { Button } from "@/components/ui/button";
import { APP_VERSION, getApiBaseUrl, getHealthApiUrl, getVersionApiUrl } from "@/lib/env";
import { ChatBubbleIcon, EyeClosedIcon, EyeIcon, LogoIcon, SettingsIcon } from "@/components/Icons";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { type AssetsQuery } from "@/gql/types";
import { ASSETS_QUERY } from "@/pages/assets/assets-query";
import { MARKET_DATA_POLL_INTERVAL } from "@/lib/apollo";

const primaryNavItems = [
  { label: "Portfolio", to: "/portfolio" },
  { label: "Activity", to: "/activity" },
  { label: "Assets", to: "/assets" },
  { label: "Accounts", to: "/accounts" },
  { label: "Settings", to: "/settings" },
];

type AssetTickerItem = {
  id: number;
  symbol: string;
  pct: string;
  tone: "positive" | "negative" | "neutral";
};

const assetTickerToneClass = {
  positive: "text-green-500 dark:text-green-400",
  negative: "text-red-500 dark:text-red-400",
  neutral: "text-muted-foreground",
} as const;

function buildAssetTickerItems(
  assets: AssetsQuery["assets"],
): AssetTickerItem[] {
  return [...assets]
    .map((asset) => {
      if (!asset.currentPrice || !asset.previousClose) return null;

      const price = Number(asset.currentPrice);
      const close = Number(asset.previousClose);
      if (Number.isNaN(price) || Number.isNaN(close) || close === 0) return null;

      const gainPct = ((price - close) / close) * 100;
      const tone =
        gainPct > 0 ? "positive" : gainPct < 0 ? "negative" : "neutral";
      const sign = gainPct > 0 ? "+" : "";
      const pct = `${sign}${gainPct.toFixed(2)}%`;

      return {
        id: asset.id,
        symbol: asset.symbol,
        pct,
        tone,
      };
    })
    .filter((item): item is AssetTickerItem => item !== null)
    .sort((a, b) => a.symbol.localeCompare(b.symbol));
}

function AssetValueTicker() {
  const { data, error, loading } = useQuery<AssetsQuery>(ASSETS_QUERY, {
    fetchPolicy: "cache-and-network",
    pollInterval: MARKET_DATA_POLL_INTERVAL,
  });

  const items = useMemo(
    () => buildAssetTickerItems(data?.assets ?? []),
    [data?.assets],
  );

  if (error || (loading && items.length === 0) || items.length === 0) {
    return null;
  }

  const tickerText = items
    .map((item) => `${item.symbol} ${item.pct}`)
    .join(" | ");

  return (
    <div
      aria-label={`Asset values: ${tickerText}`}
      className="overflow-hidden border-t border-emerald-500/20 bg-black py-1 text-emerald-400"
      data-testid="asset-value-ticker"
    >
      <div aria-hidden="true" className="asset-ticker-track">
        {[0, 1].map((copy) => (
          <div className="asset-ticker-set" key={copy}>
            {items.map((item) => (
              <span
                className="inline-flex items-center gap-2 whitespace-nowrap font-mono text-xs font-semibold uppercase tabular-nums sm:text-sm"
                key={`${copy}-${item.id}`}
              >
                <span className={assetTickerToneClass[item.tone]}>
                  {item.symbol}
                </span>
                <span className={assetTickerToneClass[item.tone]}>
                  {item.pct}
                </span>
              </span>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

export function AppShell() {
  const apolloClient = useApolloClient();
  const { hideValues, toggleHideValues } = useUiState();
  const [assistantOpen, setAssistantOpen] = useState(false);
  const [ringKey, setRingKey] = useState(0);
  const apiBaseUrl = getApiBaseUrl();
  const [backendStatus, setBackendStatus] = useState<
    "connected" | "checking" | "unavailable"
  >("checking");
  const [backendVersion, setBackendVersion] = useState<string | null>(null);

  function handleManualRefresh() {
    void apolloClient.refetchQueries({ include: ["Assets", "Portfolio", "FxRates"] });
    setRingKey((k) => k + 1);
  }

  useEffect(() => {
    let cancelled = false;

    async function checkBackendHealth() {
      setBackendStatus("checking");

      try {
        const [healthResponse, versionResponse] = await Promise.all([
          fetch(getHealthApiUrl()),
          fetch(getVersionApiUrl()),
        ]);

        if (!cancelled) {
          setBackendStatus(healthResponse.ok ? "connected" : "unavailable");
          if (versionResponse.ok) {
            const data = (await versionResponse.json()) as { version: string };
            setBackendVersion(data.version);
          }
        }
      } catch {
        if (!cancelled) {
          setBackendStatus("unavailable");
        }
      }
    }

    void checkBackendHealth();

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <>
      <div className="min-h-svh bg-muted/30">
        <header className="sticky top-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
          <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-4 px-4 py-3 sm:gap-6 sm:px-6 sm:py-4">
            <nav
              aria-label="Primary"
              className="scrollbar-hide flex-1 overflow-x-auto"
            >
              <div className="flex items-center gap-4 sm:gap-6">
                {primaryNavItems.map((item) => (
                  <NavLink
                    key={item.to}
                    className={({ isActive }) =>
                      cn(
                        "inline-flex items-center whitespace-nowrap border-b-2 px-1 py-1 text-sm font-medium transition-colors",
                        isActive
                          ? "border-foreground text-foreground"
                          : "border-transparent text-muted-foreground hover:text-foreground",
                      )
                    }
                    to={item.to}
                  >
                    {item.to === "/settings" ? <SettingsIcon className="size-4" /> : item.label}
                  </NavLink>
                ))}
              </div>
            </nav>

            <div className="flex shrink-0 flex-col items-end gap-1">
              <div className="flex items-center gap-2 sm:gap-3">
                <Button
                  aria-expanded={assistantOpen}
                  aria-haspopup="dialog"
                  aria-label="Open assistant chat"
                  className="size-9 rounded-full"
                  onClick={() => setAssistantOpen(true)}
                  size="icon"
                  type="button"
                  variant="ghost"
                >
                  <ChatBubbleIcon />
                </Button>
                <Button
                  aria-label={
                    hideValues ? "Show financial values" : "Hide financial values"
                  }
                  className="size-9 rounded-full"
                  onClick={toggleHideValues}
                  size="icon"
                  type="button"
                  variant="ghost"
                >
                  {hideValues ? <EyeClosedIcon /> : <EyeIcon />}
                </Button>
                <div className="relative size-9">
                  <button
                    aria-label="Siniscalco"
                    aria-live="polite"
                    className={cn(
                      "flex size-9 cursor-pointer items-center justify-center rounded-full shadow-sm transition-colors",
                      backendStatus === "connected" && "bg-emerald-600 text-white",
                      backendStatus === "checking" && "bg-amber-500 text-white",
                      backendStatus === "unavailable" &&
                        "bg-destructive text-destructive-foreground",
                    )}
                    onClick={handleManualRefresh}
                    title={`Backend: ${backendStatus}`}
                    type="button"
                  >
                    <LogoIcon className="size-5" />
                  </button>
                  <svg
                    key={ringKey}
                    aria-hidden="true"
                    className="pointer-events-none absolute inset-0 -rotate-90"
                    viewBox="0 0 36 36"
                  >
                    <circle
                      className="refresh-countdown-ring"
                      cx="18"
                      cy="18"
                      fill="none"
                      r="16"
                      stroke="white"
                      strokeDasharray="100.53"
                      strokeOpacity="0.5"
                      strokeWidth="2"
                      style={{ animationDuration: `${MARKET_DATA_POLL_INTERVAL}ms` }}
                    />
                  </svg>
                </div>
              </div>
              {backendStatus === "unavailable" ? (
                <span className="max-w-36 truncate text-[0.65rem] leading-none text-muted-foreground sm:max-w-56">
                  {apiBaseUrl}
                </span>
              ) : (
                <span className="text-[0.65rem] leading-none text-muted-foreground tabular-nums">
                  {backendVersion ? `api ${backendVersion}` : null}
                  {backendVersion ? " · " : null}
                  {`ui ${APP_VERSION}`}
                </span>
              )}
            </div>
          </div>
          <AssetValueTicker />
        </header>
        <div className="mx-auto w-full max-w-6xl px-4 py-8 sm:px-6">
          <Outlet />
        </div>
      </div>
      <AssistantPanel open={assistantOpen} onClose={() => setAssistantOpen(false)} />
    </>
  );
}
