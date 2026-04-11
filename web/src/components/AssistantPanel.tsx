import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { AssistantChatPanel, AssistantRuntimeBoundary, ThreadList } from "@/components/assistant";
import { Button } from "@/components/ui/button";
import {
  getAssistantModelsApiUrl,
  getAssistantReasoningEffortApiUrl,
  getAssistantSelectedModelApiUrl,
  getAssistantSystemPromptApiUrl,
} from "@/lib/env";
import { ChatBubbleIcon, CloseIcon, HistoryIcon, SettingsIcon } from "@/components/Icons";
import { useBodyScrollLock } from "@/lib/use-body-scroll-lock";
import { cn } from "@/lib/utils";

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
  reasoning: boolean;
  reasoning_effort: string;
  openai_enabled: boolean;
  last_refreshed_at: string | null;
  refresh_error: string | null;
};

const REASONING_EFFORTS = ["none", "minimal", "low", "medium", "high", "xhigh"] as const;

type SystemPromptResponse = {
  prompt: string;
  is_default: boolean;
};

type AssistantPanelProps = {
  open: boolean;
  onClose: () => void;
};

export function AssistantPanel({ open, onClose }: AssistantPanelProps) {
  const [historyOpen, setHistoryOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"chat" | "settings">("chat");
  const dialogRef = useRef<HTMLDivElement>(null);
  const [assistantModels, setAssistantModels] =
    useState<AssistantModelsResponse | null>(null);
  const [assistantModelsStatus, setAssistantModelsStatus] = useState<
    "idle" | "loading" | "saving" | "ready" | "error"
  >("idle");
  const [assistantModelsError, setAssistantModelsError] = useState<string | null>(null);
  const [systemPrompt, setSystemPrompt] = useState<SystemPromptResponse | null>(null);
  const [systemPromptDraft, setSystemPromptDraft] = useState<string>("");
  const [systemPromptStatus, setSystemPromptStatus] = useState<
    "idle" | "loading" | "saving" | "ready" | "error"
  >("idle");
  const [systemPromptError, setSystemPromptError] = useState<string | null>(null);

  useBodyScrollLock(open);

  useEffect(() => {
    if (!open) return;

    const dialog = dialogRef.current;
    if (dialog) {
      const firstFocusable = dialog.querySelector<HTMLElement>(FOCUSABLE_SELECTORS);
      firstFocusable?.focus();
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
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
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, onClose]);

  useEffect(() => {
    if (!open) return;

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
          error instanceof Error ? error.message : "Failed to load assistant models",
        );
      }
    }

    void loadAssistantModels();

    return () => {
      controller.abort();
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;

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
  }, [open]);

  async function handleAssistantModelChange(nextModel: string) {
    if (!assistantModels || nextModel === assistantModels.selected_model) return;

    setAssistantModelsStatus("saving");
    setAssistantModelsError(null);

    try {
      const response = await fetch(getAssistantSelectedModelApiUrl(), {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
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
        error instanceof Error ? error.message : "Failed to update assistant model",
      );
    }
  }

  async function handleReasoningEffortChange(nextEffort: string) {
    if (!assistantModels || nextEffort === assistantModels.reasoning_effort) return;

    setAssistantModelsStatus("saving");
    setAssistantModelsError(null);

    try {
      const response = await fetch(getAssistantReasoningEffortApiUrl(), {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ effort: nextEffort }),
      });
      const payload = (await response.json().catch(() => null)) as
        | AssistantModelsResponse
        | { error?: string }
        | null;

      if (!response.ok) {
        throw new Error(
          parseResponseError(
            payload as { error?: string } | null,
            `reasoning effort update failed with ${response.status}`,
          ),
        );
      }

      setAssistantModels(payload as AssistantModelsResponse);
      setAssistantModelsStatus("ready");
    } catch (error) {
      setAssistantModelsStatus("error");
      setAssistantModelsError(
        error instanceof Error ? error.message : "Failed to update reasoning effort",
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

  if (!open) return null;

  return createPortal(
    <div
      aria-modal="true"
      className="fixed inset-0 z-[60] flex items-end justify-center bg-black/40 p-3 backdrop-blur-sm sm:items-center sm:p-6"
      ref={dialogRef}
      role="dialog"
    >
      <AssistantRuntimeBoundary>
        <div className="flex h-[min(46rem,100%)] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border bg-background shadow-2xl">
          {/* Header */}
          <div className="flex items-center justify-between gap-2 border-b px-3 py-3 sm:gap-4 sm:px-5">
            <div className="flex min-w-0 items-center gap-2 sm:gap-4">
              <ChatBubbleIcon className="size-5 shrink-0 text-muted-foreground" />
              <select
                aria-label="Assistant model"
                className="h-8 min-w-0 max-w-[10rem] truncate rounded-lg border border-input bg-transparent px-2 text-xs outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-60"
                disabled={
                  assistantModelsStatus === "loading" ||
                  assistantModelsStatus === "saving" ||
                  assistantModels === null
                }
                onChange={(event) => void handleAssistantModelChange(event.target.value)}
                value={assistantModels?.selected_model ?? ""}
              >
                {assistantModels?.models.map((model) => (
                  <option key={model} value={model}>
                    {model}
                  </option>
                )) ?? (
                  <option value="">
                    {assistantModelsStatus === "loading"
                      ? "Loading..."
                      : "Unavailable"}
                  </option>
                )}
              </select>
              {assistantModels?.reasoning && (
                <select
                  aria-label="Reasoning effort"
                  className="h-8 rounded-lg border border-amber-300 bg-amber-50 px-2 text-xs text-amber-700 outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-60 dark:border-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
                  disabled={
                    assistantModelsStatus === "loading" ||
                    assistantModelsStatus === "saving"
                  }
                  onChange={(event) => void handleReasoningEffortChange(event.target.value)}
                  value={assistantModels.reasoning_effort}
                >
                  {REASONING_EFFORTS.map((effort) => (
                    <option key={effort} value={effort}>
                      {effort}
                    </option>
                  ))}
                </select>
              )}
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <Button
                aria-label={activeTab === "settings" ? "Hide settings" : "Show settings"}
                aria-pressed={activeTab === "settings"}
                className={cn(
                  "size-9 rounded-full",
                  activeTab === "settings" && "bg-muted text-foreground",
                )}
                onClick={() => setActiveTab(activeTab === "settings" ? "chat" : "settings")}
                size="icon"
                title={activeTab === "settings" ? "Hide settings" : "Show settings"}
                type="button"
                variant="ghost"
              >
                <SettingsIcon />
              </Button>
              <Button
                aria-label={historyOpen ? "Hide chat history" : "Show chat history"}
                aria-pressed={historyOpen}
                className={cn(
                  "size-9 rounded-full",
                  historyOpen && activeTab === "chat" && "bg-muted text-foreground",
                )}
                onClick={() => {
                  setActiveTab("chat");
                  setHistoryOpen((v) => (activeTab === "chat" ? !v : true));
                }}
                size="icon"
                title={historyOpen ? "Hide chat history" : "Show chat history"}
                type="button"
                variant="ghost"
              >
                <HistoryIcon />
              </Button>
              <Button
                aria-label="Close assistant chat"
                className="size-9 rounded-full"
                onClick={onClose}
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
              {(assistantModelsError || assistantModels?.refresh_error) && (
                <p className="text-xs text-destructive">
                  {assistantModelsError || assistantModels?.refresh_error}
                </p>
              )}

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
                    systemPromptStatus === "loading" || systemPromptStatus === "saving"
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
  );
}
