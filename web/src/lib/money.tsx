import { formatMoney, type FormatMoneyOptions } from "@/lib/format-money";
import { cn } from "@/lib/utils";

type MoneyTextProps = {
  value: number | string;
  currency?: string;
  hidden: boolean;
  className?: string;
} & FormatMoneyOptions;

export function MoneyText({
  value,
  currency,
  hidden,
  className,
  ...options
}: MoneyTextProps) {
  const formatted = formatMoney(value, currency, hidden, options);

  return (
    <span
      className={cn(
        "inline-block whitespace-nowrap font-mono tabular-nums",
        className,
      )}
      style={{ width: `${formatted.widthCh}ch` }}
    >
      {formatted.text}
    </span>
  );
}
