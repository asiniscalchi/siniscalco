import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { getAssistantChatApiUrl } from "@/lib/env";
import { ResizeObserverMock } from "@/test/browser-mocks";
import { AssistantPage } from ".";

function threadsResponse() {
  return Promise.resolve(
    new Response(JSON.stringify([]), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

describe("AssistantPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);
    window.HTMLElement.prototype.scrollTo = vi.fn();
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("renders the assistant scaffold empty state", () => {
    vi.mocked(fetch).mockImplementation((url) => {
      if (String(url).includes("/assistant/threads")) return threadsResponse();
      return Promise.reject(new Error(`Unexpected fetch: ${String(url)}`));
    });

    render(<AssistantPage />);

    expect(screen.getByRole("heading", { name: "Assistant", level: 1 })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Ask about the app", level: 3 })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Assistant message" })).toBeTruthy();
  });

  it("replies to a submitted message through the mock backend text event", async () => {
    vi.mocked(fetch).mockImplementation((url) => {
      if (String(url).includes("/assistant/threads")) {
        return threadsResponse();
      }
      // Chat endpoint — SSE stream (mock backend sends a single "text" event)
      const body =
        'data: {"type":"text","text":"The portfolio area is where the app aggregates account totals, allocations, holdings, and FX context.","model":"mock-backend"}\n\n';
      return Promise.resolve(
        new Response(body, {
          status: 200,
          headers: { "Content-Type": "text/event-stream" },
        }),
      );
    });

    render(<AssistantPage />);

    fireEvent.change(screen.getByRole("textbox", { name: "Assistant message" }), {
      target: { value: "Tell me about the portfolio page" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    expect(
      await screen.findByText(/portfolio area is where the app aggregates account totals/i),
    ).toBeTruthy();
    expect(fetch).toHaveBeenCalledWith(
      getAssistantChatApiUrl(),
      expect.objectContaining({ method: "POST" }),
    );
    expect(screen.getByText("You")).toBeTruthy();
    expect(screen.getAllByText("Assistant").length).toBeGreaterThan(0);
  });

  it("replies to a submitted message through the streaming text_delta path", async () => {
    vi.mocked(fetch).mockImplementation((url) => {
      if (String(url).includes("/assistant/threads")) {
        return threadsResponse();
      }
      // Chat endpoint — SSE stream (OpenAI path sends text_delta events)
      const body = [
        'data: {"type":"text_delta","delta":"Hello "}',
        'data: {"type":"text_delta","delta":"from streaming."}',
        "",
      ].join("\n\n");
      return Promise.resolve(
        new Response(body, {
          status: 200,
          headers: { "Content-Type": "text/event-stream" },
        }),
      );
    });

    render(<AssistantPage />);

    fireEvent.change(screen.getByRole("textbox", { name: "Assistant message" }), {
      target: { value: "Hello" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    expect(await screen.findByText(/Hello from streaming\./)).toBeTruthy();
  });

  it("renders reasoning as an expandable message part", async () => {
    vi.mocked(fetch).mockImplementation((url) => {
      if (String(url).includes("/assistant/threads")) {
        return threadsResponse();
      }
      const body = [
        'data: {"type":"reasoning_delta","delta":"Checking portfolio accounts."}',
        'data: {"type":"text_delta","delta":"Your portfolio is empty."}',
        "",
      ].join("\n\n");
      return Promise.resolve(
        new Response(body, {
          status: 200,
          headers: { "Content-Type": "text/event-stream" },
        }),
      );
    });

    render(<AssistantPage />);

    fireEvent.change(screen.getByRole("textbox", { name: "Assistant message" }), {
      target: { value: "What about my portfolio?" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    expect(await screen.findByText("Your portfolio is empty.")).toBeTruthy();
    const reasoningToggle = await screen.findByRole("button", { name: /checking portfolio accounts/i });
    expect(reasoningToggle.getAttribute("aria-expanded")).toBe("false");

    fireEvent.click(reasoningToggle);

    expect(reasoningToggle.getAttribute("aria-expanded")).toBe("true");
    const expandedPanel = document.getElementById(reasoningToggle.getAttribute("aria-controls")!);
    expect(expandedPanel).toBeTruthy();
    expect(expandedPanel!.textContent).toContain("Checking portfolio accounts.");
  });
});
