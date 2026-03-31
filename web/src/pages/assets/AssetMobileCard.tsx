import { ItemLabel } from "@/components/ItemLabel";
import { ExternalLinkIcon, PencilIcon, TrashIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";

export type AssetMobileCardChange = {
  abs: string | null;
  pct: string;
  positive: boolean;
};

type AssetMobileCardProps = {
  assetId: number;
  assetName: string;
  assetSymbol: string;
  assetType: string;
  dailyGain: AssetMobileCardChange | null;
  gain: AssetMobileCardChange | null;
  isin: string | null;
  isDeleting: boolean;
  isLocked: boolean;
  onDelete: () => void;
  onEdit: () => void;
  price: string;
  totalValue: string | null;
};

const ftMarketsUrl = (isin: string) =>
  `https://markets.ft.com/data/equities/tearsheet/summary?s=${isin}`;

export function AssetMobileCard({
  assetId,
  assetName,
  assetSymbol,
  assetType,
  dailyGain,
  gain,
  isin,
  isDeleting,
  isLocked,
  onDelete,
  onEdit,
  price,
  totalValue,
}: AssetMobileCardProps) {
  return (
    <div
      className="rounded-lg border px-3 py-2 text-sm"
      data-testid={`mobile-asset-card-${assetId}`}
    >
      <div className="flex items-start gap-3">
        <div className="grid min-w-0 flex-1 grid-cols-[minmax(0,1fr)_auto] gap-x-2 gap-y-0.5">
          <ItemLabel primary={assetSymbol} secondary={assetName} />
          {isin && (
            <div
              className="col-start-2 row-start-1 self-start text-center text-[11px] text-muted-foreground font-mono"
              data-testid={`mobile-asset-isin-${assetId}`}
            >
              <a
                className="inline-flex items-center gap-1 hover:text-foreground hover:underline"
                href={ftMarketsUrl(isin)}
                rel="noopener noreferrer"
                target="_blank"
              >
                {isin}
                <ExternalLinkIcon className="size-3 shrink-0" />
              </a>
            </div>
          )}
          <div className="mt-0.5 flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
            <span className="font-mono tabular-nums">{price}</span>
          </div>
          {dailyGain && (
            <div className={`mt-0.5 font-mono tabular-nums text-[11px] ${dailyGain.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"}`}>
              Today: {dailyGain.abs ? `${dailyGain.abs} (${dailyGain.pct})` : dailyGain.pct}
            </div>
          )}
        </div>
        <div
          className="flex shrink-0 self-stretch flex-col items-end text-right"
          data-testid={`mobile-asset-side-${assetId}`}
        >
          <span className="inline-flex items-center rounded-full border bg-muted/50 px-1.5 py-px text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
            {assetType.replace("_", " ")}
          </span>
          {totalValue && (
            <div
              className="mt-0.5 font-mono tabular-nums text-[11px] text-muted-foreground"
              data-testid={`mobile-asset-total-value-${assetId}`}
            >
              {totalValue}
            </div>
          )}
          {!isLocked && (
            <div className="mt-2 flex shrink-0 gap-0.5">
              <Button
                disabled={isDeleting}
                onClick={onEdit}
                size="icon"
                title="Edit asset"
                variant="ghost"
              >
                <PencilIcon />
                <span className="sr-only">Edit</span>
              </Button>
              <Button
                className="text-destructive hover:bg-destructive/10"
                disabled={isDeleting}
                onClick={onDelete}
                size="icon"
                title="Delete asset"
                variant="ghost"
              >
                {isDeleting ? (
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
              data-testid={`mobile-asset-gain-${assetId}`}
            >
              {gain.abs && <div>Gain: {gain.abs}</div>}
              <div data-testid={`mobile-asset-gain-pct-${assetId}`}>{gain.pct}</div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
