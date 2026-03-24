import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import {
  getAccountPositionsApiUrl,
  getAccountDetailApiUrl,
  getAssetsApiUrl,
  getCurrenciesApiUrl,
  readApiErrorMessage,
  type AssetPositionResponse,
  type AssetResponse,
  type CurrencyResponse,
} from "@/lib/api";

import { AccountDetailErrorState } from "./AccountDetailErrorState";
import { AccountDetailLoadingState } from "./AccountDetailLoadingState";
import { AccountDetailReadyState } from "./AccountDetailReadyState";
import type { AccountDetail, ReadyState } from "./types";

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

    const resolvedAccountId = accountId;
    let cancelled = false;

    async function loadAccount() {
      setRequestState({ status: "loading" });

      try {
        const [accountResponse, currenciesResponse] = await Promise.all([
          fetch(getAccountDetailApiUrl(resolvedAccountId)),
          fetch(getCurrenciesApiUrl()),
        ]);

        if (!accountResponse.ok) {
          const message = await readApiErrorMessage(
            accountResponse,
            "Could not load account.",
          );
          throw new Error(message);
        }

        if (!currenciesResponse.ok) {
          const message = await readApiErrorMessage(
            currenciesResponse,
            "Could not load currencies.",
          );
          throw new Error(message);
        }

        const [account, currencies] = await Promise.all([
          accountResponse.json() as Promise<AccountDetail>,
          currenciesResponse.json() as Promise<CurrencyResponse[]>,
        ]);

        const [positionsResult, assetsResult] = await Promise.allSettled([
          Promise.resolve().then(() =>
            fetch(getAccountPositionsApiUrl(resolvedAccountId)),
          ),
          Promise.resolve().then(() => fetch(getAssetsApiUrl())),
        ]);

        const positions =
          positionsResult.status === "fulfilled" && positionsResult.value.ok
            ? ((await positionsResult.value.json()) as AssetPositionResponse[])
            : [];

        const assets =
          assetsResult.status === "fulfilled" && assetsResult.value.ok
            ? ((await assetsResult.value.json()) as AssetResponse[])
            : [];

        const currenciesList = currencies.map((currency) => currency.code);
        const assetsById = new Map(assets.map((asset) => [asset.id, asset]));
        const accountAssets = positions.flatMap((position) => {
          const asset = assetsById.get(position.asset_id);

          if (!asset) {
            return [];
          }

          return [
            {
              asset_id: position.asset_id,
              symbol: asset.symbol,
              name: asset.name,
              asset_type: asset.asset_type,
              quantity: position.quantity,
            },
          ];
        });

        if (!cancelled) {
          setRequestState({
            status: "ready",
            data: { account, currencies: currenciesList, assets: accountAssets },
          });
        }
      } catch (error) {
        if (!cancelled) {
          setRequestState({
            status: "error",
            message:
              error instanceof Error
                ? error.message
                : "Could not load account.",
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
