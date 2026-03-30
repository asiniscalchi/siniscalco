import { useState } from "react";
import { gql } from "@apollo/client/core";
import { useMutation, useQuery } from "@apollo/client/react";

import { ItemLabel } from "@/components/ItemLabel";
import { ExternalLinkIcon, LockIcon, PencilIcon, PlusIcon, TrashIcon, UnlockIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { formatMoney } from "@/lib/format-money";
import { extractGqlErrorMessage } from "@/lib/gql";
import { type AssetsQuery } from "@/gql/types";

import { ASSETS_QUERY } from "./assets-query";

const ftMarketsUrl = (isin: string) =>
  `https://markets.ft.com/data/equities/tearsheet/summary?s=${isin}`;

const DELETE_ASSET_MUTATION = gql`
  mutation DeleteAsset($id: Int!) {
    deleteAsset(id: $id)
  }
`;

import { AssetFormModal } from "./AssetFormModal";

export function AssetsTableCard() {
  const [isLocked, setIsLocked] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [editingAsset, setEditingAsset] = useState<AssetsQuery["assets"][number] | null>(null);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  type AssetItem = AssetsQuery["assets"][number];

  const { data, loading, error, refetch } = useQuery<AssetsQuery>(ASSETS_QUERY);
  const assets = data?.assets ?? [];

  const [deleteAssetMutation] = useMutation(DELETE_ASSET_MUTATION);

  const handleDeleteClick = async (asset: AssetItem) => {
    if (!window.confirm(`Are you sure you want to delete ${asset.symbol}?`)) {
      return;
    }

    setIsDeleting(asset.id);
    try {
      await deleteAssetMutation({ variables: { id: asset.id } });
      await refetch();
    } catch (err) {
      alert(extractGqlErrorMessage(err, "Failed to delete asset"));
    } finally {
      setIsDeleting(null);
    }
  };

  const formatPrice = (asset: AssetItem) => {
    if (!asset.currentPrice || !asset.currentPriceCurrency) {
      return "Pending";
    }

    return formatMoney(asset.currentPrice, asset.currentPriceCurrency, false, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 6,
    }).text;
  };

  const formatTotalValue = (asset: AssetItem) => {
    if (!asset.convertedTotalValue || !asset.convertedTotalValueCurrency) {
      return null;
    }

    return formatMoney(
      asset.convertedTotalValue,
      asset.convertedTotalValueCurrency,
      false,
    ).text;
  };

  const formatGain = (asset: AssetItem) => {
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
  };

  const formatDailyGain = (asset: AssetItem) => {
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
  };

  const priceLabel = (asset: AssetItem) => {
    if (asset.currentPriceAsOf) {
      const parsed = new Date(asset.currentPriceAsOf);
      if (!Number.isNaN(parsed.getTime())) {
        return `Updated ${parsed.toLocaleString()}`;
      }
    }

    return asset.quoteSymbol || asset.symbol;
  };

  if (loading && assets.length === 0) {
    return <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />;
  }

  if (error && assets.length === 0) {
    return (
      <Card className="border-destructive/30 bg-background">
        <CardHeader>
          <CardTitle>Error</CardTitle>
          <CardDescription>Failed to load assets</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={() => void refetch()}>Retry</Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <>
      <Card className="min-w-0 bg-background">
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <div className="space-y-1">
            <h1 className="text-2xl font-semibold tracking-tight">Assets</h1>
          </div>
          <div className="flex items-center gap-2">
            <Button
              aria-label={isLocked ? "Unlock edit mode" : "Lock edit mode"}
              className={cn(
                "size-9 rounded-full transition-colors",
                !isLocked &&
                  "bg-amber-100 text-amber-900 hover:bg-amber-200 dark:bg-amber-900/30 dark:text-amber-400 dark:hover:bg-amber-900/50",
              )}
              onClick={() => setIsLocked((locked) => !locked)}
              size="icon"
              title={isLocked ? "Unlock edit mode" : "Lock edit mode"}
              type="button"
              variant="ghost"
            >
              {isLocked ? <LockIcon /> : <UnlockIcon />}
            </Button>
            <Button
              aria-label="Add Asset"
              onClick={() => {
                setEditingAsset(null);
                setShowModal(true);
              }}
              size="icon-lg"
              title="Add Asset"
            >
              <PlusIcon />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="min-w-0">
          {assets.length === 0 ? (
            <div className="py-12 text-center">
              <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-full bg-muted">
                <PlusIcon className="size-6 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium">No assets yet</h3>
              <p className="mb-6 text-sm text-muted-foreground">
                Add your first asset to start recording transactions.
              </p>
              <Button
                aria-label="Add Asset"
                onClick={() => {
                  setEditingAsset(null);
                  setShowModal(true);
                }}
                size="icon-lg"
                title="Add Asset"
                variant="outline"
              >
                <PlusIcon />
              </Button>
            </div>
          ) : (
            <>
              <div className="space-y-1.5 sm:hidden">
                {assets.map((asset) => {
                  const daily = formatDailyGain(asset);
                  const gain = formatGain(asset);
                  const totalValue = formatTotalValue(asset);

                  return (
                    <div
                      className="flex items-start gap-3 rounded-lg border px-3 py-2 text-sm"
                      data-testid={`mobile-asset-card-${asset.id}`}
                      key={asset.id}
                    >
                      <div className="min-w-0 flex-1">
                        <ItemLabel primary={asset.symbol} secondary={asset.name} />
                        <div className="mt-0.5 flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
                          <span className="font-mono tabular-nums">{formatPrice(asset)}</span>
                        </div>
                        {asset.isin && (
                          <div className="mt-0.5 text-[11px] text-muted-foreground font-mono">
                            <a
                              className="inline-flex items-center gap-1 hover:text-foreground hover:underline"
                              href={ftMarketsUrl(asset.isin!)}
                              rel="noopener noreferrer"
                              target="_blank"
                            >
                              {asset.isin}
                              <ExternalLinkIcon className="size-3 shrink-0" />
                            </a>
                          </div>
                        )}
                        {daily && (
                          <div className={`mt-0.5 font-mono tabular-nums text-[11px] ${daily.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"}`}>
                            Today: {daily.abs ? `${daily.abs} (${daily.pct})` : daily.pct}
                          </div>
                        )}
                      </div>
                      <div
                        className="flex shrink-0 self-stretch flex-col items-end gap-2 text-right"
                        data-testid={`mobile-asset-side-${asset.id}`}
                      >
                        <span className="inline-flex items-center rounded-full border bg-muted/50 px-1.5 py-px text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
                          {asset.assetType.replace("_", " ")}
                        </span>
                        {totalValue && (
                          <div
                            className="font-mono tabular-nums text-[11px] text-muted-foreground"
                            data-testid={`mobile-asset-total-value-${asset.id}`}
                          >
                            {totalValue}
                          </div>
                        )}
                        {!isLocked && (
                          <div className="flex shrink-0 gap-0.5">
                            <Button
                              disabled={isDeleting !== null}
                              onClick={() => {
                                setEditingAsset(asset);
                                setShowModal(true);
                              }}
                              size="icon"
                              title="Edit asset"
                              variant="ghost"
                            >
                              <PencilIcon />
                              <span className="sr-only">Edit</span>
                            </Button>
                            <Button
                              className="text-destructive hover:bg-destructive/10"
                              disabled={isDeleting !== null}
                              onClick={() => void handleDeleteClick(asset)}
                              size="icon"
                              title="Delete asset"
                              variant="ghost"
                            >
                              {isDeleting === asset.id ? (
                                <div className="size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
                              ) : (
                                <TrashIcon />
                              )}
                              <span className="sr-only">Delete</span>
                            </Button>
                          </div>
                        )}
                        {gain && (
                          <div
                            className={`mt-auto font-mono tabular-nums text-[11px] ${gain.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"}`}
                            data-testid={`mobile-asset-gain-${asset.id}`}
                          >
                            {gain.abs && <div>Gain: {gain.abs}</div>}
                            <div data-testid={`mobile-asset-gain-pct-${asset.id}`}>{gain.pct}</div>
                          </div>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>

              <div className="hidden w-full overflow-x-auto sm:block">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                      <th className="pb-3 pr-4">Asset</th>
                      <th className="pb-3 pr-4">Type</th>
                      <th className="pb-3 pr-4">Price</th>
                      <th className="pb-3 pr-4">ISIN</th>
                      <th className="pb-3 pr-4">Holdings</th>
                      <th className="pb-3 pr-4">Daily</th>
                      <th className="pb-3 pr-4">Gain</th>
                      {!isLocked && <th className="pb-3 text-right">Actions</th>}
                    </tr>
                  </thead>
                  <tbody className="divide-y">
                    {assets.map((asset) => (
                      <tr
                        className="group transition-colors hover:bg-muted/30"
                        key={asset.id}
                      >
                        <td className="py-3 pr-4">
                          <ItemLabel primary={asset.symbol} secondary={asset.name} />
                        </td>
                        <td className="py-3 pr-4">
                          <span className="inline-flex items-center rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide">
                            {asset.assetType.replace("_", " ")}
                          </span>
                        </td>
                        <td className="py-3 pr-4">
                          <div className="font-mono text-[13px] tabular-nums">
                            {formatPrice(asset)}
                          </div>
                          <div className="text-[11px] text-muted-foreground">
                            {priceLabel(asset)}
                          </div>
                        </td>
                        <td className="py-3 pr-4 font-mono text-[11px] text-muted-foreground">
                          {asset.isin ? (
                            <a
                              className="inline-flex items-center gap-1 hover:text-foreground hover:underline"
                              href={ftMarketsUrl(asset.isin!)}
                              rel="noopener noreferrer"
                              target="_blank"
                            >
                              {asset.isin}
                              <ExternalLinkIcon className="size-3 shrink-0" />
                            </a>
                          ) : "—"}
                        </td>
                        <td className="py-3 pr-4">
                          {formatTotalValue(asset) ? (
                            <>
                              <div className="font-mono text-[13px] tabular-nums">
                                {formatTotalValue(asset)}
                              </div>
                              <div className="text-[11px] text-muted-foreground font-mono tabular-nums">
                                {asset.totalQuantity ?? "—"}
                              </div>
                            </>
                          ) : (
                            <div className="font-mono text-[13px] tabular-nums">
                              {asset.totalQuantity ?? "—"}
                            </div>
                          )}
                        </td>
                        <td className="py-3 pr-4">
                          {(() => {
                            const daily = formatDailyGain(asset);
                            if (!daily) return <span className="text-muted-foreground">—</span>;
                            return (
                              <div className={daily.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"}>
                                {daily.abs && (
                                  <div className="font-mono text-[13px] tabular-nums">{daily.abs}</div>
                                )}
                                <div className="font-mono text-[11px] tabular-nums">{daily.pct}</div>
                              </div>
                            );
                          })()}
                        </td>
                        <td className="py-3 pr-4">
                          {(() => {
                            const gain = formatGain(asset);
                            if (!gain) return <span className="text-muted-foreground">—</span>;
                            return (
                              <div className={gain.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"}>
                                {gain.abs && (
                                  <div className="font-mono text-[13px] tabular-nums">{gain.abs}</div>
                                )}
                                <div className="font-mono text-[11px] tabular-nums">{gain.pct}</div>
                              </div>
                            );
                          })()}
                        </td>
                        {!isLocked && (
                          <td className="py-3 text-right">
                            <div className="flex justify-end gap-1">
                              <Button
                                disabled={isDeleting !== null}
                                onClick={() => {
                                  setEditingAsset(asset);
                                  setShowModal(true);
                                }}
                                size="icon"
                                title="Edit asset"
                                variant="ghost"
                              >
                                <PencilIcon />
                                <span className="sr-only">Edit</span>
                              </Button>
                              <Button
                                className="text-destructive hover:bg-destructive/10"
                                disabled={isDeleting !== null}
                                onClick={() => void handleDeleteClick(asset)}
                                size="icon"
                                title="Delete asset"
                                variant="ghost"
                              >
                                {isDeleting === asset.id ? (
                                  <div className="size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
                                ) : (
                                  <TrashIcon />
                                )}
                                <span className="sr-only">Delete</span>
                              </Button>
                            </div>
                          </td>
                        )}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <AssetFormModal
        key={showModal ? (editingAsset?.id ?? "new") : "closed"}
        editingAsset={editingAsset}
        open={showModal}
        onClose={() => {
          setShowModal(false);
          setEditingAsset(null);
        }}
        onSaved={() => {
          setShowModal(false);
          setEditingAsset(null);
          void refetch();
        }}
      />
    </>
  );
}
