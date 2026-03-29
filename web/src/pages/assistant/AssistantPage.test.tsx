import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AssistantPage } from ".";

class ResizeObserverMock {
  observe() {}

  unobserve() {}

  disconnect() {}
}

describe("AssistantPage", () => {
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);
    window.HTMLElement.prototype.scrollTo = vi.fn();
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("renders the assistant scaffold empty state", () => {
    render(<AssistantPage />);

    expect(screen.getByRole("heading", { name: "Assistant", level: 1 })).toBeTruthy();
    expect(screen.getByText("Assistant Workspace")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Ask about the app", level: 3 })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Assistant message" })).toBeTruthy();
  });

  it("replies to a submitted message through the local assistant-ui runtime", async () => {
    render(<AssistantPage />);

    fireEvent.change(screen.getByRole("textbox", { name: "Assistant message" }), {
      target: { value: "Tell me about the portfolio page" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    expect(
      await screen.findByText(/portfolio area is where the app aggregates account totals/i),
    ).toBeTruthy();
    expect(screen.getByText("You")).toBeTruthy();
    expect(screen.getAllByText("Assistant").length).toBeGreaterThan(0);
  });
});
