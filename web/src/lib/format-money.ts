const HIDDEN_MONEY_MASK = "••••";

type FormatMoneyOptions = {
  includeCurrency?: boolean;
  minimumFractionDigits?: number;
  maximumFractionDigits?: number;
};

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
  const visibleText =
    includeCurrency && currency
      ? `${formattedNumber} ${currency}`
      : formattedNumber;
  const hiddenText =
    includeCurrency && currency
      ? `${HIDDEN_MONEY_MASK} ${currency}`
      : HIDDEN_MONEY_MASK;

  return {
    text: hidden ? hiddenText : visibleText,
    widthCh: Math.max(visibleText.length, hiddenText.length),
  };
}

export { formatMoney, HIDDEN_MONEY_MASK };
export type { FormatMoneyOptions };
