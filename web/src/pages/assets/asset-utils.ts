import { formatMoney } from "@/lib/format-money";
import { type AssetsQuery } from "@/gql/types";

export type AssetItem = AssetsQuery["assets"][number];

export type GainResult = {
  pct: string;
  abs: string | null;
  positive: boolean;
};

const PROVIDER_LABELS: Record<string, string> = {
  alpha_vantage: "Alpha Vantage",
  coincap: "CoinCap",
  coingecko: "CoinGecko",
  eodhd: "EODHD",
  fcsapi: "FCS API",
  finnhub: "Finnhub",
  fmp: "FMP",
  itick: "iTick",
  marketstack: "Marketstack",
  polygon: "Polygon",
  tiingo: "Tiingo",
  twelve_data: "Twelve Data",
  yahoo: "Yahoo",
};

export function formatProviderName(provider: string): string {
  return PROVIDER_LABELS[provider] ?? provider;
}

export function formatPrice(asset: AssetItem): string {
  if (!asset.currentPrice || !asset.currentPriceCurrency) {
    return "Pending";
  }

  return formatMoney(asset.currentPrice, asset.currentPriceCurrency, false, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).text;
}

export function formatTotalValue(asset: AssetItem): string | null {
  if (!asset.convertedTotalValue || !asset.convertedTotalValueCurrency) {
    return null;
  }

  return formatMoney(asset.convertedTotalValue, asset.convertedTotalValueCurrency, false).text;
}

export function formatGain(asset: AssetItem): GainResult | null {
  const { convertedTotalValue, convertedTotalCostBasis, convertedTotalValueCurrency } = asset;
  if (!convertedTotalValue || !convertedTotalCostBasis) return null;

  const value = Number(convertedTotalValue);
  const costTotal = Number(convertedTotalCostBasis);
  if (Number.isNaN(value) || Number.isNaN(costTotal) || costTotal === 0) return null;

  const gainAbs = value - costTotal;
  const gainPct = (gainAbs / costTotal) * 100;
  const positive = gainAbs >= 0;
  const sign = positive ? "+" : "-";

  return {
    pct: `${positive ? "+" : ""}${gainPct.toFixed(2)}%`,
    abs: `${sign}${formatMoney(Math.abs(gainAbs), convertedTotalValueCurrency ?? undefined, false).text}`,
    positive,
  };
}

export function formatDailyGain(asset: AssetItem): GainResult | null {
  const { currentPrice, currentPriceCurrency, previousClose, previousCloseCurrency } = asset;
  if (!currentPrice || !previousClose) return null;

  const price = Number(currentPrice);
  const close = Number(previousClose);
  if (Number.isNaN(price) || Number.isNaN(close) || close === 0) return null;

  const gainPct = ((price - close) / close) * 100;
  const sameCurrency = currentPriceCurrency && previousCloseCurrency === currentPriceCurrency;
  const gainAbs = sameCurrency ? price - close : null;

  const sign = gainPct >= 0 ? "+" : "";
  const pct = `${sign}${gainPct.toFixed(2)}%`;
  const abs = gainAbs !== null
    ? `${sign}${formatMoney(gainAbs, currentPriceCurrency ?? undefined, false).text}`
    : null;

  return { pct, abs, positive: gainPct >= 0 };
}

export function dailyGainPctRaw(asset: AssetItem): number | null {
  const { currentPrice, previousClose } = asset;
  if (!currentPrice || !previousClose) return null;
  const price = Number(currentPrice);
  const close = Number(previousClose);
  if (Number.isNaN(price) || Number.isNaN(close) || close === 0) return null;
  return ((price - close) / close) * 100;
}

export function totalGainPctRaw(asset: AssetItem): number | null {
  const { convertedTotalValue, convertedTotalCostBasis } = asset;
  if (!convertedTotalValue || !convertedTotalCostBasis) return null;
  const value = Number(convertedTotalValue);
  const cost = Number(convertedTotalCostBasis);
  if (Number.isNaN(value) || Number.isNaN(cost) || cost === 0) return null;
  return ((value - cost) / cost) * 100;
}

export function priceLabel(asset: AssetItem): string {
  if (asset.currentPriceAsOf) {
    const isoDate = asset.currentPriceAsOf.match(/^\d{4}-\d{2}-\d{2}/)?.[0];
    if (isoDate) {
      return `Updated ${isoDate}`;
    }
  }

  return asset.quoteSymbol || asset.symbol;
}

export function quoteSourceLabel(asset: AssetItem): string | null {
  if (asset.quoteSourceSymbol && asset.quoteSourceProvider) {
    return `${asset.quoteSourceSymbol} via ${formatProviderName(asset.quoteSourceProvider)}`;
  }

  if (asset.quoteSourceSymbol) {
    return asset.quoteSourceSymbol;
  }

  return null;
}

export function quoteSourceUpdatedLabel(asset: AssetItem): string | null {
  const timestamp = asset.quoteSourceLastSuccessAt ?? asset.currentPriceAsOf;
  const isoDate = timestamp?.match(/^\d{4}-\d{2}-\d{2}/)?.[0];

  return isoDate ? `Detected ${isoDate}` : null;
}

export const yahooFinanceUrl = (symbol: string) =>
  `https://finance.yahoo.com/quote/${encodeURIComponent(symbol)}`;

export const coinMarketCapUrl = (name: string) =>
  `https://coinmarketcap.com/currencies/${encodeURIComponent(
    name.trim().toLowerCase().replace(/\s+/g, "-"),
  )}/`;

export function assetExternalUrl(asset: {
  assetType: AssetItem["assetType"];
  symbol: string;
  name: string;
  quoteSymbol?: string | null;
}): string {
  if (asset.assetType === "CRYPTO") {
    return coinMarketCapUrl(asset.name);
  }
  return yahooFinanceUrl(asset.quoteSymbol ?? asset.symbol);
}

export function priceHealthLabel(assets: AssetItem[]): string {
  const priced = assets.filter((asset) => asset.currentPrice && asset.currentPriceCurrency).length;
  const pending = assets.length - priced;
  const detectedSources = assets.filter(
    (asset) => asset.quoteSourceSymbol || asset.quoteSourceProvider,
  ).length;

  const parts = [`${priced} priced`];
  if (pending > 0) parts.push(`${pending} pending`);
  if (detectedSources > 0) parts.push(`${detectedSources} detected source${detectedSources === 1 ? "" : "s"}`);

  return `Prices: ${parts.join(" · ")}`;
}
