import { PencilIcon, PlusIcon, TrashIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { formatMoney } from "@/lib/format-money";

import type { AssetResponse } from "@/lib/api";

type AssetsTableCardProps = {
  assets: AssetResponse[];
  isLocked: boolean;
  isDeleting: number | null;
  onCreateClick: () => void;
  onEditClick: (asset: AssetResponse) => void;
  onDeleteClick: (asset: AssetResponse) => void;
};

export function AssetsTableCard({
  assets,
  isLocked,
  isDeleting,
  onCreateClick,
  onEditClick,
  onDeleteClick,
}: AssetsTableCardProps) {
  const formatPrice = (asset: AssetResponse) => {
    if (!asset.current_price || !asset.current_price_currency) {
      return "Pending";
    }

    return formatMoney(asset.current_price, asset.current_price_currency, false, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 6,
    }).text;
  };

  const priceLabel = (asset: AssetResponse) => {
    if (asset.current_price_as_of) {
      const parsed = new Date(asset.current_price_as_of);
      if (!Number.isNaN(parsed.getTime())) {
        return `Updated ${parsed.toLocaleString()}`;
      }
    }

    return asset.quote_symbol || asset.symbol;
  };

  return (
    <Card className="min-w-0 bg-background">
      <CardContent className="min-w-0 pt-6">
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
              onClick={onCreateClick}
              size="icon-lg"
              title="Add Asset"
              variant="outline"
            >
              <PlusIcon />
            </Button>
          </div>
        ) : (
          <div className="w-full overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left font-semibold text-[11px] uppercase tracking-wider text-muted-foreground">
                  <th className="pb-3 pr-4">Symbol</th>
                  <th className="pb-3 pr-4">Name</th>
                  <th className="pb-3 pr-4">Type</th>
                  <th className="pb-3 pr-4">Price</th>
                  <th className="pb-3 pr-4">ISIN</th>
                  {!isLocked && <th className="pb-3 text-right">Actions</th>}
                </tr>
              </thead>
              <tbody className="divide-y">
                {assets.map((asset) => (
                  <tr
                    className="group transition-colors hover:bg-muted/30"
                    key={asset.id}
                  >
                    <td className="py-3 pr-4 font-bold tabular-nums">
                      {asset.symbol}
                    </td>
                    <td className="py-3 pr-4">{asset.name}</td>
                    <td className="py-3 pr-4">
                      <span className="inline-flex items-center rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide">
                        {asset.asset_type.replace("_", " ")}
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
                      {asset.isin || "—"}
                    </td>
                    {!isLocked && (
                      <td className="py-3 text-right">
                        <div className="flex justify-end gap-1">
                          <Button
                            disabled={isDeleting !== null}
                            onClick={() => onEditClick(asset)}
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
                            onClick={() => onDeleteClick(asset)}
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
        )}
      </CardContent>
    </Card>
  );
}
