import { ItemLabel } from "@/components/ItemLabel";
import { type AssetType } from "@/gql/types";
import { assetExternalUrl } from "./asset-utils";

export function AssetLabel({
  symbol,
  name,
  quoteSymbol,
  assetType,
  className,
  noLink,
}: {
  symbol: string;
  name: string;
  quoteSymbol?: string | null;
  assetType?: AssetType;
  className?: string;
  noLink?: boolean;
}) {
  if (noLink) {
    return <ItemLabel className={className} primary={symbol} secondary={name} />;
  }

  return (
    <ItemLabel
      className={className}
      href={assetExternalUrl({ symbol, name, quoteSymbol, assetType: assetType ?? "STOCK" })}
      primary={symbol}
      secondary={name}
      target="_blank"
    />
  );
}
