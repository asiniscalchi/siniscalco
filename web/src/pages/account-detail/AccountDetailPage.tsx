import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";
import { useNavigate, useParams } from "react-router-dom";

import { extractGqlErrorMessage } from "@/lib/gql";
import { type AccountQuery } from "@/gql/types";

const ACCOUNT_QUERY = gql`
  query Account($id: Int!) {
    account(id: $id) {
      id name accountType baseCurrency summaryStatus createdAt
      cashTotalAmount assetTotalAmount totalAmount totalCurrency
      balances { currency amount updatedAt }
    }
  }
`;

import { AccountDetailErrorState } from "./AccountDetailErrorState";
import { AccountDetailLoadingState } from "./AccountDetailLoadingState";
import { AccountDetailReadyState } from "./AccountDetailReadyState";

export function AccountDetailPage() {
  const { accountId } = useParams<{ accountId: string }>();
  const navigate = useNavigate();
  const numericId = accountId ? parseInt(accountId) : 0;

  const { data, loading, error, refetch } = useQuery<AccountQuery>(
    ACCOUNT_QUERY,
    { variables: { id: numericId }, skip: !accountId },
  );

  if (!accountId) {
    return (
      <AccountDetailErrorState
        message="Account not found."
        onRetry={() => void refetch()}
      />
    );
  }

  if (loading) {
    return <AccountDetailLoadingState />;
  }

  if (error) {
    return (
      <AccountDetailErrorState
        message={extractGqlErrorMessage(error, "Could not load account.")}
        onRetry={() => void refetch()}
      />
    );
  }

  const account = data?.account;
  if (!account) {
    return (
      <AccountDetailErrorState
        message="Could not load account."
        onRetry={() => void refetch()}
      />
    );
  }

  return (
    <AccountDetailReadyState
      account={account}
      onDeleteSuccess={() => navigate("/accounts")}
    />
  );
}
