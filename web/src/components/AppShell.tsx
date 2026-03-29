import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { NavLink, Outlet } from "react-router-dom";

import { AssistantChat } from "@/components/assistant";
import { Button } from "@/components/ui/button";
import {
  getAssistantModelsApiUrl,
  getAssistantSelectedModelApiUrl,
  getHealthApiUrl,
} from "@/lib/env";
import {
  ChatBubbleIcon,
  CloseIcon,
  EyeClosedIcon,
  EyeIcon,
  LogoIcon,
} from "@/components/Icons";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

const primaryNavItems = [
  { label: "Portfolio", to: "/portfolio" },
  { label: "Accounts", to: "/accounts" },
  { label: "Assets", to: "/assets" },
  { label: "Transactions", to: "/transactions" },
  { label: "Transfers", to: "/transfers" },
];

const FOCUSABLE_SELECTORS =
  'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])';

function parseResponseError(
  payload: { error?: string } | null,
  fallback: string,
): string {
  return payload && "error" in payload && payload.error
    ? payload.error
    : fallback;
}

type AssistantModelsResponse = {
  models: string[];
  selected_model: string;
  openai_enabled: boolean;
  last_refreshed_at: string | null;
  refresh_error: string | null;
};

export function AppShell() {
  const { hideValues, toggleHideValues } = useUiState();
  const [assistantOpen, setAssistantOpen] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);
  const [assistantModels, setAssistantModels] =
    useState<AssistantModelsResponse | null>(null);
  const [assistantModelsStatus, setAssistantModelsStatus] = useState<
    "idle" | "loading" | "saving" | "ready" | "error"
  >("idle");
  const [assistantModelsError, setAssistantModelsError] = useState<string | null>(
    null,
  );
  const [backendStatus, setBackendStatus] = useState<
    "connected" | "checking" | "unavailable"
  >("checking");

  useEffect(() => {
    let cancelled = false;

    async function checkBackendHealth() {
      setBackendStatus("checking");

      try {
        const response = await fetch(getHealthApiUrl());

        if (!cancelled) {
          setBackendStatus(response.ok ? "connected" : "unavailable");
        }
      } catch {
        if (!cancelled) {
          setBackendStatus("unavailable");
        }
      }
    }

    void checkBackendHealth();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!assistantOpen) {
      document.body.style.overflow = "";
      return;
    }

    document.body.style.overflow = "hidden";

    const dialog = dialogRef.current;
    if (dialog) {
      const firstFocusable = dialog.querySelector<HTMLElement>(FOCUSABLE_SELECTORS);
      firstFocusable?.focus();
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setAssistantOpen(false);
        return;
      }

      if (event.key !== "Tab" || !dialog) return;

      const focusable = Array.from(
        dialog.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTORS),
      );
      if (focusable.length === 0) return;

      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (event.shiftKey) {
        if (document.activeElement === first) {
          event.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = "";
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [assistantOpen]);

  useEffect(() => {
    if (!assistantOpen) return;

    const controller = new AbortController();

    async function loadAssistantModels() {
      setAssistantModelsStatus("loading");
      setAssistantModelsError(null);

      try {
        const response = await fetch(getAssistantModelsApiUrl(), {
          signal: controller.signal,
        });
        const payload = (await response.json().catch(() => null)) as
          | AssistantModelsResponse
          | { error?: string }
          | null;

        if (!response.ok) {
          throw new Error(
            parseResponseError(
              payload as { error?: string } | null,
              `assistant model request failed with ${response.status}`,
            ),
          );
        }

        setAssistantModels(payload as AssistantModelsResponse);
        setAssistantModelsStatus("ready");
      } catch (error) {
        if (controller.signal.aborted) return;
        setAssistantModelsStatus("error");
        setAssistantModelsError(
          error instanceof Error
            ? error.message
            : "Failed to load assistant models",
        );
      }
    }

    void loadAssistantModels();

    return () => {
      controller.abort();
    };
  }, [assistantOpen]);

  async function handleAssistantModelChange(nextModel: string) {
    if (!assistantModels || nextModel === assistantModels.selected_model) {
      return;
    }

    setAssistantModelsStatus("saving");
    setAssistantModelsError(null);

    try {
      const response = await fetch(getAssistantSelectedModelApiUrl(), {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ model: nextModel }),
      });
      const payload = (await response.json().catch(() => null)) as
        | AssistantModelsResponse
        | { error?: string }
        | null;

      if (!response.ok) {
        throw new Error(
          parseResponseError(
            payload as { error?: string } | null,
            `assistant model update failed with ${response.status}`,
          ),
        );
      }

      setAssistantModels(payload as AssistantModelsResponse);
      setAssistantModelsStatus("ready");
    } catch (error) {
      setAssistantModelsStatus("error");
      setAssistantModelsError(
        error instanceof Error
          ? error.message
          : "Failed to update assistant model",
      );
    }
  }

  return (
    <>
      <div className="min-h-svh bg-muted/30">
        <header className="sticky top-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
          <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-4 px-4 py-3 sm:gap-6 sm:px-6 sm:py-4">
            <div className="flex shrink-0 items-center gap-2">
              <div
                aria-label="Siniscalco"
                aria-live="polite"
                className={cn(
                  "flex size-9 items-center justify-center rounded-xl shadow-sm transition-colors",
                  backendStatus === "connected" && "bg-emerald-600 text-white",
                  backendStatus === "checking" && "bg-amber-500 text-white",
                  backendStatus === "unavailable" &&
                    "bg-destructive text-destructive-foreground",
                )}
                role="img"
                title={`Backend: ${backendStatus}`}
              >
                <LogoIcon className="size-5" />
                <span className="sr-only">Backend {backendStatus}</span>
              </div>
            </div>

            <nav
              aria-label="Primary"
              className="scrollbar-hide flex-1 overflow-x-auto"
            >
              <div className="flex items-center gap-4 sm:gap-6">
                {primaryNavItems.map((item) => (
                  <NavLink
                    key={item.to}
                    className={({ isActive }) =>
                      cn(
                        "inline-flex items-center whitespace-nowrap border-b-2 px-1 py-1 text-sm font-medium transition-colors",
                        isActive
                          ? "border-foreground text-foreground"
                          : "border-transparent text-muted-foreground hover:text-foreground",
                      )
                    }
                    to={item.to}
                  >
                    {item.label}
                  </NavLink>
                ))}
              </div>
            </nav>

            <div className="flex shrink-0 items-center gap-2 sm:gap-3">
              <Button
                aria-expanded={assistantOpen}
                aria-haspopup="dialog"
                aria-label="Open assistant chat"
                className="size-9 rounded-full"
                onClick={() => setAssistantOpen(true)}
                size="icon"
                type="button"
                variant="ghost"
              >
                <ChatBubbleIcon />
              </Button>
              <Button
                aria-label={
                  hideValues ? "Show financial values" : "Hide financial values"
                }
                className="size-9 rounded-full"
                onClick={toggleHideValues}
                size="icon"
                type="button"
                variant="ghost"
              >
                {hideValues ? <EyeClosedIcon /> : <EyeIcon />}
              </Button>
            </div>
          </div>
        </header>
        <div className="mx-auto w-full max-w-6xl px-4 py-8 sm:px-6">
          <Outlet />
        </div>
      </div>
      {assistantOpen
        ? createPortal(
            <div
              aria-modal="true"
              className="fixed inset-0 z-[60] flex items-end justify-center bg-black/40 p-3 backdrop-blur-sm sm:items-center sm:p-6"
              ref={dialogRef}
              role="dialog"
            >
              <div className="flex h-[min(46rem,100%)] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border bg-background shadow-2xl">
                <div className="flex items-start justify-between gap-4 border-b px-5 py-4">
                  <div className="min-w-0 flex-1 space-y-3">
                    <div className="space-y-1">
                      <h2 className="text-base font-semibold">Assistant</h2>
                      <p className="text-sm text-muted-foreground">
                        Popup chat entrypoint for quick questions inside the app.
                      </p>
                    </div>

                    <div className="max-w-xs space-y-1.5">
                      <label
                        className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground"
                        htmlFor="assistant-model"
                      >
                        Model
                      </label>
                      <select
                        aria-label="Assistant model"
                        className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-60"
                        disabled={
                          assistantModelsStatus === "loading" ||
                          assistantModelsStatus === "saving" ||
                          assistantModels === null
                        }
                        id="assistant-model"
                        onChange={(event) =>
                          void handleAssistantModelChange(event.target.value)
                        }
                        value={assistantModels?.selected_model ?? ""}
                      >
                        {assistantModels?.models.map((model) => (
                          <option key={model} value={model}>
                            {model}
                          </option>
                        )) ?? (
                          <option value="">
                            {assistantModelsStatus === "loading"
                              ? "Loading models..."
                              : "Models unavailable"}
                          </option>
                        )}
                      </select>
                      <p className="text-xs text-muted-foreground">
                        {assistantModels === null
                          ? "Loading available models..."
                          : assistantModels.openai_enabled
                          ? `Active model: ${assistantModels.selected_model}`
                          : "Backend mock assistant active"}
                      </p>
                      {assistantModelsError || assistantModels?.refresh_error ? (
                        <p className="text-xs text-destructive">
                          {assistantModelsError || assistantModels?.refresh_error}
                        </p>
                      ) : null}
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <Button
                      aria-label="Close assistant chat"
                      className="size-9 rounded-full"
                      onClick={() => setAssistantOpen(false)}
                      size="icon"
                      type="button"
                      variant="ghost"
                    >
                      <CloseIcon />
                    </Button>
                  </div>
                </div>

                <AssistantChat className="min-h-0 flex-1" />
              </div>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
