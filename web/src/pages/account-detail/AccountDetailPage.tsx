import { useQuery } from "@apollo/client/react";
import { useNavigate, useParams } from "react-router-dom";

import {
  ACCOUNT_POSITIONS_QUERY,
  ACCOUNT_QUERY,
  ASSETS_QUERY,
  CURRENCIES_QUERY,
  FX_RATES_QUERY,
  extractGqlErrorMessage,
  type FxRateSummary,
} from "@/lib/api";

import { AccountDetailErrorState } from "./AccountDetailErrorState";
import { AccountDetailLoadingState } from "./AccountDetailLoadingState";
import { AccountDetailReadyState } from "./AccountDetailReadyState";
import type { ReadyState } from "./types";

function computeAssetValue(
  quantity: string,
  price: string | null,
  priceCurrency: string | null,
  baseCurrency: string,
  fxRates: FxRateSummary | null,
): string | null {
  if (!price || !priceCurrency) return null;

  const rawValue = parseFloat(quantity) * parseFloat(price);

  if (priceCurrency === baseCurrency) return rawValue.toFixed(2);
  if (!fxRates) return null;

  const { targetCurrency, rates } = fxRates;

  const rateToTarget = (currency: string): number | null => {
    if (currency === targetCurrency) return 1;
    const entry = rates.find((r) => r.currency === currency);
    return entry ? parseFloat(entry.rate) : null;
  };

  const priceRate = rateToTarget(priceCurrency);
  const baseRate = rateToTarget(baseCurrency);

  if (priceRate === null || baseRate === null) return null;

  return (rawValue * (priceRate / baseRate)).toFixed(2);
}

export function AccountDetailPage() {
  const { accountId } = useParams<{ accountId: string }>();
  const navigate = useNavigate();
  const numericId = accountId ? parseInt(accountId) : 0;

  const { data: accountData, loading: accountLoading, error: accountError, refetch } = useQuery<{ account: ReadyState["account"] }>(
    ACCOUNT_QUERY,
    { variables: { id: numericId }, skip: !accountId },
  );

  const { data: currenciesData, loading: currenciesLoading } = useQuery<{ currencies: string[] }>(
    CURRENCIES_QUERY,
    { skip: !accountId },
  );

  const skipSecondary = !accountId || accountLoading || !!accountError;

  const { data: positionsData } = useQuery<{ accountPositions: Array<{ accountId: number; assetId: number; quantity: string }> }>(
    ACCOUNT_POSITIONS_QUERY,
    { variables: { accountId: numericId }, skip: skipSecondary },
  );

  const { data: assetsData } = useQuery<{ assets: Array<{ id: number; symbol: string; name: string; assetType: string; currentPrice: string | null; currentPriceCurrency: string | null }> }>(
    ASSETS_QUERY,
    { skip: skipSecondary },
  );

  const { data: fxData } = useQuery<{ fxRates: FxRateSummary }>(
    FX_RATES_QUERY,
    { skip: skipSecondary },
  );

  if (!accountId) {
    return (
      <AccountDetailErrorState
        message="Account not found."
        onRetry={() => void refetch()}
      />
    );
  }

  if (accountLoading || currenciesLoading) {
    return <AccountDetailLoadingState />;
  }

  if (accountError) {
    return (
      <AccountDetailErrorState
        message={extractGqlErrorMessage(accountError, "Could not load account.")}
        onRetry={() => void refetch()}
      />
    );
  }

  const account = accountData?.account;
  if (!account) {
    return (
      <AccountDetailErrorState
        message="Could not load account."
        onRetry={() => void refetch()}
      />
    );
  }

  const currencies = currenciesData?.currencies ?? [];
  const positions = positionsData?.accountPositions ?? [];
  const assets = assetsData?.assets ?? [];
  const fxRates = fxData?.fxRates ?? null;

  const assetsById = new Map(assets.map((asset) => [asset.id, asset]));
  const accountAssets = positions.flatMap((position) => {
    const asset = assetsById.get(position.assetId);
    if (!asset) return [];
    return [
      {
        assetId: position.assetId,
        symbol: asset.symbol,
        name: asset.name,
        assetType: asset.assetType,
        quantity: position.quantity,
        value: computeAssetValue(
          position.quantity,
          asset.currentPrice,
          asset.currentPriceCurrency,
          account.baseCurrency,
          fxRates,
        ),
      },
    ];
  });

  return (
    <AccountDetailReadyState
      account={account}
      assets={accountAssets}
      currencies={currencies}
      onDeleteSuccess={() => navigate("/accounts")}
      onRefresh={() => void refetch()}
    />
  );
}
