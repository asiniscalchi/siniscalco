import { ItemLabel } from "@/components/ItemLabel";
import { yahooFinanceUrl } from "./asset-utils";

export function AssetLabel({
  symbol,
  name,
  quoteSymbol,
  className,
  noLink,
}: {
  symbol: string;
  name: string;
  quoteSymbol?: string | null;
  className?: string;
  noLink?: boolean;
}) {
  if (noLink) {
    return <ItemLabel className={className} primary={symbol} secondary={name} />;
  }

  return (
    <ItemLabel
      className={className}
      href={yahooFinanceUrl(quoteSymbol ?? symbol)}
      primary={symbol}
      secondary={name}
      target="_blank"
    />
  );
}
