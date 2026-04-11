import { formatMoney } from "@/lib/format-money";
import { type AssetsQuery } from "@/gql/types";

export type AssetItem = AssetsQuery["assets"][number];

export type GainResult = {
  pct: string;
  abs: string | null;
  positive: boolean;
};

export function formatPrice(asset: AssetItem): string {
  if (!asset.currentPrice || !asset.currentPriceCurrency) {
    return "Pending";
  }

  return formatMoney(asset.currentPrice, asset.currentPriceCurrency, false, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
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

export function priceLabel(asset: AssetItem): string {
  if (asset.currentPriceAsOf) {
    const isoDate = asset.currentPriceAsOf.match(/^\d{4}-\d{2}-\d{2}/)?.[0];
    if (isoDate) {
      return `Updated ${isoDate}`;
    }
  }

  return asset.quoteSymbol || asset.symbol;
}
