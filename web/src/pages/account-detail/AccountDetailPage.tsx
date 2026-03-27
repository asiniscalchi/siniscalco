import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import {
  fetchAccount,
  fetchAccountPositions,
  fetchAssets,
  fetchCurrencies,
  fetchFxRates,
  extractGqlErrorMessage,
  type FxRateSummary,
} from "@/lib/api";

import { AccountDetailErrorState } from "./AccountDetailErrorState";
import { AccountDetailLoadingState } from "./AccountDetailLoadingState";
import { AccountDetailReadyState } from "./AccountDetailReadyState";
import type { AccountDetail, ReadyState } from "./types";

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
  const [requestState, setRequestState] = useState<
    | { status: "loading" }
    | { status: "error"; message: string }
    | { status: "ready"; data: ReadyState }
  >({ status: "loading" });
  const [retryToken, setRetryToken] = useState(0);

  useEffect(() => {
    if (!accountId) {
      setRequestState({ status: "error", message: "Account not found." });
      return;
    }

    const resolvedAccountId = parseInt(accountId);
    let cancelled = false;

    async function loadAccount() {
      setRequestState({ status: "loading" });

      try {
        const [account, currencies] = await Promise.all([
          fetchAccount(resolvedAccountId),
          fetchCurrencies(),
        ]);

        const [positionsResult, assetsResult, fxRatesResult] = await Promise.allSettled([
          fetchAccountPositions(resolvedAccountId),
          fetchAssets(),
          fetchFxRates(),
        ]);

        const positions =
          positionsResult.status === "fulfilled" ? positionsResult.value : [];

        const assets =
          assetsResult.status === "fulfilled" ? assetsResult.value : [];

        const fxRates =
          fxRatesResult.status === "fulfilled" ? fxRatesResult.value : null;

        const assetsById = new Map(assets.map((asset) => [asset.id, asset]));
        const accountAssets = positions.flatMap((position) => {
          const asset = assetsById.get(position.assetId);

          if (!asset) {
            return [];
          }

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

        if (!cancelled) {
          setRequestState({
            status: "ready",
            data: { account, currencies, assets: accountAssets },
          });
        }
      } catch (error) {
        if (!cancelled) {
          setRequestState({
            status: "error",
            message: extractGqlErrorMessage(error, "Could not load account."),
          });
        }
      }
    }

    void loadAccount();

    return () => {
      cancelled = true;
    };
  }, [accountId, retryToken]);

  if (requestState.status === "loading") {
    return <AccountDetailLoadingState />;
  }

  if (requestState.status === "error") {
    return (
      <AccountDetailErrorState
        message={requestState.message}
        onRetry={() => setRetryToken((value) => value + 1)}
      />
    );
  }

  return (
    <AccountDetailReadyState
      account={requestState.data.account}
      assets={requestState.data.assets}
      currencies={requestState.data.currencies}
      onDeleteSuccess={() => navigate("/accounts")}
      onRefresh={() => setRetryToken((value) => value + 1)}
    />
  );
}
