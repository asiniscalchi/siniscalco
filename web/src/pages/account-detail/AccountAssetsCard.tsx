import { gql } from "@apollo/client/core";
import { useQuery } from "@apollo/client/react";

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { ItemLabel } from "@/components/ItemLabel";
import { type AccountPositionsQuery, type AccountAssetsQuery, type AccountFxRatesQuery } from "@/gql/types";

const ACCOUNT_POSITIONS_QUERY = gql`
  query AccountPositions($accountId: Int!) {
    accountPositions(accountId: $accountId) {
      accountId assetId quantity
    }
  }
`;

const ASSETS_QUERY = gql`
  query AccountAssets {
    assets {
      id symbol name assetType
      currentPrice currentPriceCurrency
    }
  }
`;

const FX_RATES_QUERY = gql`
  query AccountFxRates {
    fxRates {
      targetCurrency
      rates { currency rate }
    }
  }
`;
import { MoneyText } from "@/lib/money";
import { useUiState } from "@/lib/ui-state";

type AccountAsset = {
  assetId: number;
  symbol: string;
  name: string;
  assetType: string;
  quantity: string;
  value: string | null;
};

function computeAssetValue(
  quantity: string,
  price: string | null,
  priceCurrency: string | null,
  baseCurrency: string,
  fxRates: AccountFxRatesQuery["fxRates"] | null,
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

type AccountAssetsCardProps = {
  accountId: number;
  baseCurrency: string;
};

export function AccountAssetsCard({ accountId, baseCurrency }: AccountAssetsCardProps) {
  const { hideValues } = useUiState();

  const { data: positionsData } = useQuery<AccountPositionsQuery>(ACCOUNT_POSITIONS_QUERY, { variables: { accountId } });
  const { data: assetsData } = useQuery<AccountAssetsQuery>(ASSETS_QUERY, { pollInterval: 5 * 60 * 1000 });
  const { data: fxData } = useQuery<AccountFxRatesQuery>(FX_RATES_QUERY, { pollInterval: 5 * 60 * 1000 });

  const positions = positionsData?.accountPositions ?? [];
  const assets = assetsData?.assets ?? [];
  const fxRates = fxData?.fxRates ?? null;

  const assetsById = new Map(assets.map((asset) => [asset.id, asset]));
  const accountAssets: AccountAsset[] = positions.flatMap((position) => {
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
          baseCurrency,
          fxRates,
        ),
      },
    ];
  });

  return (
    <section className="space-y-4">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold tracking-tight">Assets</h2>
        <p className="text-sm text-muted-foreground">
          Current asset positions held in this account.
        </p>
      </div>

      {accountAssets.length === 0 ? (
        <Card className="border-dashed bg-background">
          <CardHeader>
            <CardTitle>No assets yet</CardTitle>
            <CardDescription>
              This account does not have any open asset positions yet.
            </CardDescription>
          </CardHeader>
        </Card>
      ) : (
        <Card className="bg-background">
          <CardContent className="pt-6">
            <div className="space-y-1.5 sm:hidden">
              {accountAssets.map((asset) => (
                <div
                  className="flex items-center gap-3 rounded-lg border px-3 py-2 text-sm"
                  key={asset.assetId}
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline justify-between gap-2">
                      <ItemLabel primary={asset.symbol} secondary={asset.name} />
                    </div>
                    <div className="mt-0.5 flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
                      <span className="inline-flex items-center rounded-full border bg-muted/50 px-1.5 py-px font-medium uppercase tracking-wide">
                        {asset.assetType.replace("_", " ")}
                      </span>
                      <div className="flex items-center gap-2 font-mono tabular-nums">
                        <span>{parseFloat(asset.quantity)}</span>
                        {asset.value ? (
                          <MoneyText
                            currency={baseCurrency}
                            hidden={hideValues}
                            value={asset.value}
                          />
                        ) : (
                          <span>—</span>
                        )}
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            <div className="hidden w-full overflow-x-auto sm:block">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                    <th className="pb-3 pr-4">Asset</th>
                    <th className="pb-3 pr-4">Type</th>
                    <th className="pb-3 pr-4 text-right">Quantity</th>
                    <th className="pb-3 text-right">Value</th>
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {accountAssets.map((asset) => (
                    <tr key={asset.assetId}>
                      <td className="py-3 pr-4">
                        <ItemLabel primary={asset.symbol} secondary={asset.name} />
                      </td>
                      <td className="py-3 pr-4">
                        <span className="inline-flex items-center rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide">
                          {asset.assetType.replace("_", " ")}
                        </span>
                      </td>
                      <td className="py-3 pr-4 text-right font-mono tabular-nums">
                        {parseFloat(asset.quantity)}
                      </td>
                      <td className="py-3 text-right">
                        {asset.value ? (
                          <MoneyText
                            currency={baseCurrency}
                            hidden={hideValues}
                            value={asset.value}
                          />
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}
    </section>
  );
}
