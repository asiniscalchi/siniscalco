const HIDDEN_MONEY_MASK = "••••";

type FormatMoneyOptions = {
  includeCurrency?: boolean;
  minimumFractionDigits?: number;
  maximumFractionDigits?: number;
};

function getCurrencySymbol(currency: string): string {
  try {
    return (
      new Intl.NumberFormat("en-US", {
        style: "currency",
        currency,
        currencyDisplay: "narrowSymbol",
      })
        .formatToParts(0)
        .find((p) => p.type === "currency")?.value ?? currency
    );
  } catch {
    return currency;
  }
}

function formatMoney(
  value: number | string,
  currency: string | undefined,
  hidden: boolean,
  {
    includeCurrency = true,
    minimumFractionDigits = 2,
    maximumFractionDigits = 2,
  }: FormatMoneyOptions = {},
) {
  const numericValue = typeof value === "string" ? Number(value) : value;
  const formattedNumber = Number.isNaN(numericValue)
    ? String(value)
    : new Intl.NumberFormat("en-US", {
        minimumFractionDigits,
        maximumFractionDigits,
      }).format(numericValue);

  const symbol = currency ? getCurrencySymbol(currency) : undefined;
  // Use prefix format when a distinct symbol exists (€2.50), suffix when only
  // the code is available (2.50 CHF).
  const prefixSymbol = symbol && symbol !== currency ? symbol : undefined;
  const suffixCode = !prefixSymbol && currency ? currency : undefined;

  const visibleText =
    includeCurrency && (prefixSymbol ?? suffixCode)
      ? prefixSymbol
        ? `${prefixSymbol}${formattedNumber}`
        : `${formattedNumber} ${suffixCode}`
      : formattedNumber;
  const hiddenText =
    includeCurrency && (prefixSymbol ?? suffixCode)
      ? prefixSymbol
        ? `${prefixSymbol}${HIDDEN_MONEY_MASK}`
        : `${HIDDEN_MONEY_MASK} ${suffixCode}`
      : HIDDEN_MONEY_MASK;

  return {
    text: hidden ? hiddenText : visibleText,
    widthCh: Math.max(visibleText.length, hiddenText.length),
  };
}

export { formatMoney, HIDDEN_MONEY_MASK };
export type { FormatMoneyOptions };
