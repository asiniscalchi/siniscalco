import { useEffect, useRef } from "react";

const YAHOO_TO_TV_EXCHANGE: Record<string, string> = {
  ".MI": "MIL",
  ".L": "LSE",
  ".DE": "XETRA",
  ".PA": "EURONEXT",
  ".AS": "EURONEXT",
  ".BR": "EURONEXT",
  ".SW": "SIX",
  ".TO": "TSX",
  ".HK": "HKEX",
  ".T": "TSE",
  ".AX": "ASX",
  ".MC": "BME",
  ".LS": "EURONEXT",
  ".OL": "OSL",
  ".ST": "OMX",
  ".HE": "OMX",
  ".CO": "OMX",
};

function toTradingViewSymbol(symbol: string, assetType: string): string {
  if (assetType === "CRYPTO") {
    // BTC-USD -> BTCUSD
    return symbol.replace("-", "").replace("/", "");
  }
  for (const [suffix, exchange] of Object.entries(YAHOO_TO_TV_EXCHANGE)) {
    if (symbol.endsWith(suffix)) {
      return `${exchange}:${symbol.slice(0, -suffix.length)}`;
    }
  }
  return symbol;
}

const CHART_HEIGHT = 300;

interface TradingViewChartProps {
  symbol: string;
  assetType: string;
}

export function TradingViewChart({ symbol, assetType }: TradingViewChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const tvSymbol = toTradingViewSymbol(symbol, assetType);
  const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    container.innerHTML = "";

    const script = document.createElement("script");
    script.src =
      "https://s3.tradingview.com/external-embedding/embed-widget-advanced-chart.js";
    script.type = "text/javascript";
    script.async = true;
    script.innerHTML = JSON.stringify({
      width: "100%",
      height: CHART_HEIGHT,
      symbol: tvSymbol,
      interval: "D",
      timezone: "Etc/UTC",
      theme: isDark ? "dark" : "light",
      style: "1",
      locale: "en",
      allow_symbol_change: false,
      calendar: false,
      support_host: "https://www.tradingview.com",
    });
    container.appendChild(script);

    return () => {
      container.innerHTML = "";
    };
  }, [tvSymbol, isDark]);

  return (
    <div
      className="tradingview-widget-container"
      ref={containerRef}
      style={{ height: `${CHART_HEIGHT}px`, width: "100%" }}
    />
  );
}
