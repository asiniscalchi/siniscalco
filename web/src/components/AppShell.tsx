import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { NavLink, Outlet } from "react-router-dom";

import { AssistantChatPanel, AssistantRuntimeBoundary, ThreadList } from "@/components/assistant";
import { ItemLabel } from "@/components/ItemLabel";
import { Button } from "@/components/ui/button";
import {
  getAssistantModelsApiUrl,
  getAssistantSelectedModelApiUrl,
  getAssistantSystemPromptApiUrl,
  getHealthApiUrl,
} from "@/lib/env";
import {
  ChatBubbleIcon,
  CloseIcon,
  EyeClosedIcon,
  EyeIcon,
  HistoryIcon,
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

type SystemPromptResponse = {
  prompt: string;
  is_default: boolean;
};

export function AppShell() {
  const { hideValues, toggleHideValues } = useUiState();
  const [assistantOpen, setAssistantOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"chat" | "settings">("chat");
  const dialogRef = useRef<HTMLDivElement>(null);
  const [assistantModels, setAssistantModels] =
    useState<AssistantModelsResponse | null>(null);
  const [assistantModelsStatus, setAssistantModelsStatus] = useState<
    "idle" | "loading" | "saving" | "ready" | "error"
  >("idle");
  const [assistantModelsError, setAssistantModelsError] = useState<string | null>(
    null,
  );
  const [systemPrompt, setSystemPrompt] = useState<SystemPromptResponse | null>(null);
  const [systemPromptDraft, setSystemPromptDraft] = useState<string>("");
  const [systemPromptStatus, setSystemPromptStatus] = useState<
    "idle" | "loading" | "saving" | "ready" | "error"
  >("idle");
  const [systemPromptError, setSystemPromptError] = useState<string | null>(null);
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

  useEffect(() => {
    if (!assistantOpen) return;

    const controller = new AbortController();

    async function loadSystemPrompt() {
      setSystemPromptStatus("loading");
      setSystemPromptError(null);

      try {
        const response = await fetch(getAssistantSystemPromptApiUrl(), {
          signal: controller.signal,
        });
        const payload = (await response.json().catch(() => null)) as
          | SystemPromptResponse
          | { error?: string }
          | null;

        if (!response.ok) {
          throw new Error(
            parseResponseError(
              payload as { error?: string } | null,
              `system prompt request failed with ${response.status}`,
            ),
          );
        }

        const data = payload as SystemPromptResponse;
        setSystemPrompt(data);
        setSystemPromptDraft(data.prompt);
        setSystemPromptStatus("ready");
      } catch (error) {
        if (controller.signal.aborted) return;
        setSystemPromptStatus("error");
        setSystemPromptError(
          error instanceof Error ? error.message : "Failed to load system prompt",
        );
      }
    }

    void loadSystemPrompt();

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

  async function handleSaveSystemPrompt() {
    if (!systemPromptDraft.trim()) return;

    setSystemPromptStatus("saving");
    setSystemPromptError(null);

    try {
      const response = await fetch(getAssistantSystemPromptApiUrl(), {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ prompt: systemPromptDraft }),
      });
      const payload = (await response.json().catch(() => null)) as
        | SystemPromptResponse
        | { error?: string }
        | null;

      if (!response.ok) {
        throw new Error(
          parseResponseError(
            payload as { error?: string } | null,
            `system prompt update failed with ${response.status}`,
          ),
        );
      }

      const data = payload as SystemPromptResponse;
      setSystemPrompt(data);
      setSystemPromptDraft(data.prompt);
      setSystemPromptStatus("ready");
    } catch (error) {
      setSystemPromptStatus("error");
      setSystemPromptError(
        error instanceof Error ? error.message : "Failed to save system prompt",
      );
    }
  }

  async function handleResetSystemPrompt() {
    setSystemPromptStatus("saving");
    setSystemPromptError(null);

    try {
      const response = await fetch(getAssistantSystemPromptApiUrl(), {
        method: "DELETE",
      });
      const payload = (await response.json().catch(() => null)) as
        | SystemPromptResponse
        | { error?: string }
        | null;

      if (!response.ok) {
        throw new Error(
          parseResponseError(
            payload as { error?: string } | null,
            `system prompt reset failed with ${response.status}`,
          ),
        );
      }

      const data = payload as SystemPromptResponse;
      setSystemPrompt(data);
      setSystemPromptDraft(data.prompt);
      setSystemPromptStatus("ready");
    } catch (error) {
      setSystemPromptStatus("error");
      setSystemPromptError(
        error instanceof Error ? error.message : "Failed to reset system prompt",
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
              <AssistantRuntimeBoundary>
                <div className="flex h-[min(46rem,100%)] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border bg-background shadow-2xl">
                  {/* Header */}
                  <div className="flex items-center justify-between gap-4 border-b px-5 py-3">
                    <div className="flex items-center gap-4">
                      <ItemLabel
                        primary="Assistant"
                        secondary={assistantModels?.selected_model}
                      />
                      <div className="flex items-center gap-1 rounded-lg bg-muted p-1">
                        <button
                          className={cn(
                            "rounded-md px-3 py-1 text-sm font-medium transition-colors",
                            activeTab === "chat"
                              ? "bg-background text-foreground shadow-sm"
                              : "text-muted-foreground hover:text-foreground",
                          )}
                          onClick={() => setActiveTab("chat")}
                          type="button"
                        >
                          Chat
                        </button>
                        <button
                          className={cn(
                            "rounded-md px-3 py-1 text-sm font-medium transition-colors",
                            activeTab === "settings"
                              ? "bg-background text-foreground shadow-sm"
                              : "text-muted-foreground hover:text-foreground",
                          )}
                          onClick={() => setActiveTab("settings")}
                          type="button"
                        >
                          Settings
                        </button>
                      </div>
                    </div>
                    <div className="flex shrink-0 items-center gap-2">
                      {activeTab === "chat" && (
                        <Button
                          aria-label={historyOpen ? "Hide chat history" : "Show chat history"}
                          aria-pressed={historyOpen}
                          className={cn(
                            "size-9 rounded-full",
                            historyOpen && "bg-muted text-foreground",
                          )}
                          onClick={() => setHistoryOpen((v) => !v)}
                          size="icon"
                          title={historyOpen ? "Hide chat history" : "Show chat history"}
                          type="button"
                          variant="ghost"
                        >
                          <HistoryIcon />
                        </Button>
                      )}
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

                  {/* Chat tab */}
                  {activeTab === "chat" && (
                    <div className="flex min-h-0 flex-1">
                      {historyOpen && (
                        <div className="w-52 shrink-0 border-r bg-muted/20 p-3">
                          <ThreadList
                            className="h-full"
                            onSelect={() => setHistoryOpen(false)}
                          />
                        </div>
                      )}
                      <AssistantChatPanel className="min-h-0 flex-1" />
                    </div>
                  )}

                  {/* Settings tab */}
                  {activeTab === "settings" && (
                    <div className="flex flex-1 flex-col gap-6 overflow-y-auto px-5 py-5">
                      <div className="space-y-1.5">
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
                        {assistantModelsError || assistantModels?.refresh_error ? (
                          <p className="text-xs text-destructive">
                            {assistantModelsError || assistantModels?.refresh_error}
                          </p>
                        ) : null}
                      </div>

                      <div className="flex flex-1 flex-col gap-1.5">
                        <label
                          className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground"
                          htmlFor="assistant-system-prompt"
                        >
                          System prompt
                        </label>
                        <textarea
                          className="flex-1 w-full resize-none rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-60"
                          disabled={
                            systemPromptStatus === "loading" ||
                            systemPromptStatus === "saving"
                          }
                          id="assistant-system-prompt"
                          onChange={(e) => setSystemPromptDraft(e.target.value)}
                          value={systemPromptStatus === "loading" ? "Loading..." : systemPromptDraft}
                        />
                        <div className="flex items-center gap-2">
                          <Button
                            disabled={
                              systemPromptStatus === "loading" ||
                              systemPromptStatus === "saving" ||
                              !systemPromptDraft.trim() ||
                              systemPromptDraft === systemPrompt?.prompt
                            }
                            onClick={() => void handleSaveSystemPrompt()}
                            size="sm"
                            type="button"
                            variant="default"
                          >
                            {systemPromptStatus === "saving" ? "Saving..." : "Save"}
                          </Button>
                          <Button
                            disabled={
                              systemPromptStatus === "loading" ||
                              systemPromptStatus === "saving" ||
                              systemPrompt?.is_default === true
                            }
                            onClick={() => void handleResetSystemPrompt()}
                            size="sm"
                            type="button"
                            variant="outline"
                          >
                            Reset to default
                          </Button>
                        </div>
                        {systemPromptError ? (
                          <p className="text-xs text-destructive">{systemPromptError}</p>
                        ) : null}
                      </div>
                    </div>
                  )}
                </div>
              </AssistantRuntimeBoundary>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
