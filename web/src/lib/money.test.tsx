import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MoneyText } from "@/lib/money";
import { formatMoney } from "@/lib/format-money";

describe("MoneyText", () => {
  it("does not set a fixed width for visible values", () => {
    const { container } = render(
      <MoneyText value="769531.12" currency="EUR" hidden={false} />,
    );
    const span = container.firstElementChild as HTMLElement;
    // A fixed ch-based width is narrower than the rendered text (€ and
    // separators are not exactly 1ch wide) and makes the value overflow onto
    // adjacent elements. Visible text must size naturally.
    expect(span.style.width).toBe("");
  });

  it("sets a fixed width for hidden values to keep totals aligned", () => {
    const { container } = render(
      <MoneyText value="769531.12" currency="EUR" hidden={true} />,
    );
    const span = container.firstElementChild as HTMLElement;
    const { widthCh } = formatMoney("769531.12", "EUR", true);
    expect(span.style.width).toBe(`${widthCh}ch`);
  });
});