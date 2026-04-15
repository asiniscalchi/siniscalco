import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { ItemLabel } from "./ItemLabel";

describe("ItemLabel", () => {
  afterEach(() => {
    cleanup();
  });

  it("allows secondary text to truncate within narrow flex layouts", () => {
    render(
      <div className="flex w-24">
        <ItemLabel
          primary="VWCE"
          secondary="Vanguard FTSE All-World UCITS ETF USD Accumulating"
        />
      </div>,
    );

    const secondary = screen.getByText(
      "Vanguard FTSE All-World UCITS ETF USD Accumulating",
    );
    const label = secondary.parentElement;

    expect(label?.className).toContain("min-w-0");
    expect(label?.className).toContain("max-w-full");
    expect(secondary.className).toContain("truncate");
    expect(secondary.className).toContain("max-w-full");
    expect(secondary.getAttribute("title")).toBe(
      "Vanguard FTSE All-World UCITS ETF USD Accumulating",
    );
  });
});
