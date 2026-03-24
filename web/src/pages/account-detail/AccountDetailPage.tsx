import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import {
  getAccountDetailApiUrl,
  getCurrenciesApiUrl,
  readApiErrorMessage,
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

        const account = (await accountResponse.json()) as AccountDetail;
        const currencies = (
          (await currenciesResponse.json()) as CurrencyResponse[]
        ).map((currency) => currency.code);

        if (!cancelled) {
          setRequestState({ status: "ready", data: { account, currencies } });
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
      currencies={requestState.data.currencies}
      onDeleteSuccess={() => navigate("/accounts")}
      onRefresh={() => setRetryToken((value) => value + 1)}
    />
  );
}
