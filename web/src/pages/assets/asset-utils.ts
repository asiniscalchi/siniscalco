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
  const { currentPrice, currentPriceCurrency, totalQuantity, avgCostBasis, avgCostBasisCurrency } =
    asset;
  if (!currentPrice || !avgCostBasis || !totalQuantity) return null;

  const price = Number(currentPrice);
  const cost = Number(avgCostBasis);
  const qty = Number(totalQuantity);
  if (Number.isNaN(price) || Number.isNaN(cost) || Number.isNaN(qty) || cost === 0) return null;

  const gainPct = ((price - cost) / cost) * 100;
  const sameCurrency = currentPriceCurrency && avgCostBasisCurrency === currentPriceCurrency;
  const gainAbs = sameCurrency ? (price - cost) * qty : null;

  const sign = gainPct >= 0 ? "+" : "";
  const pct = `${sign}${gainPct.toFixed(2)}%`;
  const abs = gainAbs !== null
    ? `${sign}${formatMoney(gainAbs, currentPriceCurrency ?? undefined, false).text}`
    : null;

  return { pct, abs, positive: gainPct >= 0 };
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
  const { currentPrice, avgCostBasis, totalQuantity } = asset;
  if (!currentPrice || !avgCostBasis || !totalQuantity) return null;
  const price = Number(currentPrice);
  const cost = Number(avgCostBasis);
  const qty = Number(totalQuantity);
  if (Number.isNaN(price) || Number.isNaN(cost) || Number.isNaN(qty) || cost === 0) return null;
  return ((price - cost) / cost) * 100;
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
