import { useEffect, useId, useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  AssistantRuntimeProvider,
  ComposerPrimitive,
  MessagePartPrimitive,
  MessagePrimitive,
  RuntimeAdapterProvider,
  ThreadPrimitive,
  useAui,
  useLocalRuntime,
  useRemoteThreadListRuntime,
  type ChatModelAdapter,
  type ReasoningMessagePart,
  type ReasoningMessagePartProps,
  type TextMessagePart,
  type ThreadMessageLike,
  type ToolCallMessagePart,
  type ToolCallMessagePartProps,
} from "@assistant-ui/react";
import { MarkdownTextPrimitive } from "@assistant-ui/react-markdown";
import type { ThreadHistoryAdapter } from "@assistant-ui/core";
import { createAssistantStream } from "assistant-stream";

import { getAssistantChatApiUrl } from "@/lib/env";
import { cn } from "@/lib/utils";
import {
  apiAppendMessage,
  apiCreateThread,
  apiDeleteThread,
  apiGetThread,
  apiListThreads,
  apiLoadMessages,
  apiRenameThread,
  apiUpdateThreadStatus,
} from "@/lib/threads-api";

// ── Chat model adapter ────────────────────────────────────────────────────────

type AssistantChatApiMessage =
  | { role: string; content: string }
  | { role: "assistant"; content: string | null; tool_calls: OpenAiToolCall[] }
  | { role: "tool"; tool_call_id: string; content: string };

type OpenAiToolCall = {
  id: string;
  type: "function";
  function: { name: string; arguments: string };
};

type AssistantChatApiErrorResponse = { error?: string };

type ChatStreamEvent =
  | { type: "tool_call"; id: string; name: string; args: Record<string, unknown> }
  | { type: "tool_result"; id: string; result: unknown }
  | { type: "text_delta"; delta: string }
  | { type: "reasoning_delta"; delta: string }
  | { type: "text"; text: string; model: string }
  | { type: "error"; error: string };

type AssistantContentPart = ToolCallMessagePart | ReasoningMessagePart | TextMessagePart;

function buildAssistantContent(
  toolCalls: ToolCallMessagePart[],
  reasoning: string,
  text: string,
): AssistantContentPart[] {
  const content: AssistantContentPart[] = [...toolCalls];
  if (reasoning) content.push({ type: "reasoning", text: reasoning });
  if (text) content.push({ type: "text", text });
  return content;
}

function extractMessageText(message: ThreadMessageLike): string {
  if (typeof message.content === "string") return message.content.trim();
  if (!Array.isArray(message.content)) return "";
  return message.content
    .flatMap((part) => (part.type !== "text" ? [] : typeof part.text === "string" ? [part.text] : []))
    .join(" ")
    .trim();
}

function serializeMessages(messages: readonly ThreadMessageLike[]): AssistantChatApiMessage[] {
  return messages.flatMap((m): AssistantChatApiMessage[] => {
    if (m.role === "user") {
      const text = extractMessageText(m);
      if (!text) return [];
      return [{ role: "user", content: text }];
    }

    if (m.role === "assistant") {
      const parts = Array.isArray(m.content) ? m.content : [];
      const toolCallParts = parts.filter((p): p is ToolCallMessagePart => p.type === "tool-call");
      const text = extractMessageText(m);

      if (toolCallParts.length > 0) {
        const toolCallMsg: AssistantChatApiMessage = {
          role: "assistant",
          content: text || null,
          tool_calls: toolCallParts.map((tc) => ({
            id: tc.toolCallId,
            type: "function" as const,
            function: {
              name: tc.toolName,
              arguments: tc.argsText ?? JSON.stringify(tc.args),
            },
          })),
        };
        const toolResultMsgs: AssistantChatApiMessage[] = toolCallParts
          .filter((tc) => tc.result !== undefined)
          .map((tc) => ({
            role: "tool" as const,
            tool_call_id: tc.toolCallId,
            content: typeof tc.result === "string" ? tc.result : JSON.stringify(tc.result),
          }));
        return [toolCallMsg, ...toolResultMsgs];
      }

      if (!text) return [];
      return [{ role: "assistant", content: text }];
    }

    return [];
  });
}

const STREAM_READ_TIMEOUT_MS = 45_000;

function readWithTimeout<T>(
  reader: ReadableStreamDefaultReader<T>,
  timeoutMs: number,
  signal: AbortSignal,
): Promise<ReadableStreamReadResult<T>> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      reader.cancel().catch(() => {});
      reject(new Error("assistant response timed out — no data received for 45 seconds"));
    }, timeoutMs);

    const onAbort = () => {
      clearTimeout(timer);
      reader.cancel().catch(() => {});
      reject(signal.reason ?? new DOMException("Aborted", "AbortError"));
    };
    signal.addEventListener("abort", onAbort, { once: true });

    reader.read().then(
      (result) => {
        clearTimeout(timer);
        signal.removeEventListener("abort", onAbort);
        resolve(result);
      },
      (err) => {
        clearTimeout(timer);
        signal.removeEventListener("abort", onAbort);
        reject(err);
      },
    );
  });
}

const assistantModelAdapter: ChatModelAdapter = {
  async *run({ messages, abortSignal }) {
    if (abortSignal.aborted) return;

    const response = await fetch(getAssistantChatApiUrl(), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ messages: serializeMessages(messages) }),
      signal: abortSignal,
    });

    if (!response.ok) {
      const payload = (await response.json().catch(() => null)) as AssistantChatApiErrorResponse | null;
      throw new Error(payload?.error ?? `assistant backend request failed with ${response.status}`);
    }

    const body = response.body;
    if (!body) throw new Error("assistant backend returned an empty response body");
    const reader = body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    const toolCalls: ToolCallMessagePart[] = [];
    let accumulatedReasoning = "";
    let accumulatedText = "";

    while (true) {
      const { done, value } = await readWithTimeout(reader, STREAM_READ_TIMEOUT_MS, abortSignal);
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";

      for (const line of lines) {
        if (!line.startsWith("data: ")) continue;
        let event: ChatStreamEvent;
        try {
          event = JSON.parse(line.slice(6)) as ChatStreamEvent;
        } catch {
          continue;
        }

        if (event.type === "tool_call") {
          toolCalls.push({
            type: "tool-call",
            toolCallId: event.id,
            toolName: event.name,
            args: event.args as ToolCallMessagePart["args"],
            argsText: JSON.stringify(event.args),
          });
          yield { content: [...toolCalls] };
        } else if (event.type === "tool_result") {
          const idx = toolCalls.findIndex((t) => t.toolCallId === event.id);
          if (idx >= 0) {
            toolCalls[idx] = { ...toolCalls[idx], result: event.result };
            yield { content: [...toolCalls] };
          }
        } else if (event.type === "reasoning_delta") {
          accumulatedReasoning += event.delta;
          yield { content: buildAssistantContent(toolCalls, accumulatedReasoning, accumulatedText) };
        } else if (event.type === "text_delta") {
          accumulatedText += event.delta;
          yield { content: buildAssistantContent(toolCalls, accumulatedReasoning, accumulatedText) };
        } else if (event.type === "text") {
          accumulatedText = event.text;
          yield { content: buildAssistantContent(toolCalls, accumulatedReasoning, accumulatedText) };
        } else if (event.type === "error") {
          throw new Error(event.error);
        }
      }
    }
  },
};

// ── Thread history provider ───────────────────────────────────────────────────

function ThreadHistoryProvider({ children }: { children: ReactNode }) {
  const aui = useAui();

  const historyAdapter: ThreadHistoryAdapter = useMemo(
    () => ({
      async load() {
        const remoteId = aui.threadListItem().getState().remoteId;
        if (!remoteId) return { messages: [] };
        return apiLoadMessages(remoteId);
      },
      async append(item) {
        const { remoteId } = await aui.threadListItem().initialize();
        await apiAppendMessage(remoteId, item);
      },
    }),
    [aui],
  );

  const adapters = useMemo(() => ({ history: historyAdapter }), [historyAdapter]);

  return <RuntimeAdapterProvider adapters={adapters}>{children}</RuntimeAdapterProvider>;
}

// ── Remote thread list runtime hook ──────────────────────────────────────────

function useAssistantRuntime() {
  return useLocalRuntime(assistantModelAdapter);
}

function useThreadListRuntime() {
  return useRemoteThreadListRuntime({
    runtimeHook: useAssistantRuntime,
    adapter: {
      async list() {
        const threads = await apiListThreads();
        return {
          threads: threads.map((t) => ({
            remoteId: t.id,
            externalId: undefined,
            status: t.status,
            title: t.title ?? undefined,
          })),
        };
      },

      async initialize(threadId) {
        await apiCreateThread(threadId);
        return { remoteId: threadId, externalId: undefined };
      },

      async fetch(threadId) {
        const t = await apiGetThread(threadId);
        return {
          remoteId: t.id,
          externalId: undefined,
          status: t.status,
          title: t.title ?? undefined,
        };
      },

      async rename(remoteId, newTitle) {
        await apiRenameThread(remoteId, newTitle);
      },

      async archive(remoteId) {
        await apiUpdateThreadStatus(remoteId, "archived");
      },

      async unarchive(remoteId) {
        await apiUpdateThreadStatus(remoteId, "regular");
      },

      async delete(remoteId) {
        await apiDeleteThread(remoteId);
      },

      async generateTitle(remoteId, messages) {
        // Derive title from the first user message (truncated to 60 chars)
        const firstUser = messages.find((m) => m.role === "user");
        const text = firstUser ? extractMessageText(firstUser) : "";
        const title = text.length > 60 ? `${text.slice(0, 57)}...` : text;
        if (title) {
          await apiRenameThread(remoteId, title);
        }
        return createAssistantStream((ctrl) => {
          if (title) ctrl.appendText(title);
        });
      },

      unstable_Provider: ThreadHistoryProvider,
    },
  });
}

// ── Runtime boundary ──────────────────────────────────────────────────────────

export function AssistantRuntimeBoundary({ children }: Readonly<{ children: ReactNode }>) {
  const runtime = useThreadListRuntime();
  return <AssistantRuntimeProvider runtime={runtime}>{children}</AssistantRuntimeProvider>;
}

// ── Message components (unchanged) ───────────────────────────────────────────

function MessageText() {
  return (
    <MessagePartPrimitive.Text
      className="text-sm leading-6 whitespace-pre-wrap"
      component="p"
      smooth={false}
    />
  );
}

function UserMessage() {
  return (
    <MessagePrimitive.Root className="flex flex-col items-end gap-2">
      <span className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
        You
      </span>
      <div className="max-w-[85%] rounded-2xl rounded-br-md bg-foreground px-4 py-3 text-sm text-background shadow-sm">
        <MessagePrimitive.Parts components={{ Text: MessageText }} />
      </div>
    </MessagePrimitive.Root>
  );
}

function AssistantMessageText() {
  return (
    <MarkdownTextPrimitive
      className="prose prose-sm max-w-none dark:prose-invert leading-6"
      smooth={false}
    />
  );
}

function AssistantThinking() {
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    const id = setInterval(() => setElapsed((s) => s + 1), 1_000);
    return () => clearInterval(id);
  }, []);

  const remaining = Math.max(0, Math.ceil(STREAM_READ_TIMEOUT_MS / 1_000) - elapsed);

  return (
    <div className="flex items-center gap-2 py-1">
      <div className="flex items-center gap-0.5">
        <span className="h-1.5 w-1.5 rounded-full bg-muted-foreground animate-bounce [animation-delay:-0.3s]" />
        <span className="h-1.5 w-1.5 rounded-full bg-muted-foreground animate-bounce [animation-delay:-0.15s]" />
        <span className="h-1.5 w-1.5 rounded-full bg-muted-foreground animate-bounce" />
      </div>
      <span className="text-xs tabular-nums text-muted-foreground">{remaining}s</span>
    </div>
  );
}

function ToolCallDisplay({ toolName, result }: ToolCallMessagePartProps) {
  const isPending = result === undefined;
  return (
    <div className="flex items-center gap-2 rounded-lg border bg-muted/40 px-3 py-1.5 text-xs text-muted-foreground">
      <span className="font-medium text-foreground">{toolName}</span>
      {isPending ? (
        <span className="italic">running…</span>
      ) : (
        <span className="text-green-600 dark:text-green-400">done</span>
      )}
    </div>
  );
}

function ReasoningDisplay({ text, status }: ReasoningMessagePartProps) {
  const [open, setOpen] = useState(false);
  const bodyId = useId();
  const isRunning = status.type === "running";

  return (
    <div className="overflow-hidden rounded-lg border bg-muted/40 text-xs text-muted-foreground">
      <button
        aria-controls={bodyId}
        aria-expanded={open}
        className="flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left"
        onClick={() => setOpen((value) => !value)}
        type="button"
      >
        <span className="font-medium text-foreground">Reasoning</span>
        <span>{isRunning ? "thinking..." : open ? "hide" : "show"}</span>
      </button>
      {open && (
        <div
          className="border-t bg-background/70 px-3 py-2 text-xs leading-5 whitespace-pre-wrap text-foreground"
          id={bodyId}
        >
          {text || "Reasoning is still streaming."}
        </div>
      )}
    </div>
  );
}

function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="flex flex-col items-start gap-2">
      <span className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
        Assistant
      </span>
      <div className="max-w-[85%] rounded-2xl rounded-bl-md border bg-background px-4 py-3 text-sm shadow-sm">
        <MessagePrimitive.Parts
          components={{
            Empty: AssistantThinking,
            Reasoning: ReasoningDisplay,
            Text: AssistantMessageText,
            tools: { Fallback: ToolCallDisplay },
          }}
        />
      </div>
    </MessagePrimitive.Root>
  );
}

// ── Chat panel ────────────────────────────────────────────────────────────────

export function AssistantChatPanel({ className }: { className?: string }) {
  return (
    <div className={cn("flex min-h-0 flex-1 flex-col", className)}>
      <ThreadPrimitive.Root className="flex min-h-0 flex-1 flex-col">
        <ThreadPrimitive.Viewport className="flex-1 space-y-6 overflow-y-auto bg-muted/20 px-4 py-5">
          <ThreadPrimitive.Empty>
            <div className="flex h-full min-h-[18rem] flex-col items-start justify-center gap-4">
              <div className="space-y-2">
                <p className="text-sm font-medium uppercase tracking-[0.2em] text-muted-foreground">
                  Ready
                </p>
                <h3 className="text-2xl font-semibold tracking-tight">Ask about the app</h3>
              </div>
              <p className="max-w-xl text-sm leading-6 text-muted-foreground">
                Ask about the portfolio, accounts, assets, transactions, or transfers. The backend
                answers from the current portfolio data snapshot and uses the active assistant model
                selected in the popup header.
              </p>
            </div>
          </ThreadPrimitive.Empty>

          <ThreadPrimitive.Messages components={{ UserMessage, AssistantMessage }} />
        </ThreadPrimitive.Viewport>

        <div className="border-t bg-background p-4">
          <ComposerPrimitive.Root className="flex items-end gap-3 rounded-2xl border bg-background p-3 shadow-sm">
            <ComposerPrimitive.Input
              aria-label="Assistant message"
              className="min-h-12 flex-1 resize-none bg-transparent px-1 py-2 text-sm leading-6 outline-none placeholder:text-muted-foreground"
              placeholder="Ask about the portfolio app..."
              rows={1}
            />
            <ThreadPrimitive.If running={false}>
              <ComposerPrimitive.Send className="inline-flex h-11 shrink-0 items-center justify-center rounded-xl bg-foreground px-4 text-sm font-medium text-background transition-colors hover:bg-foreground/90 disabled:cursor-not-allowed disabled:opacity-50">
                Send
              </ComposerPrimitive.Send>
            </ThreadPrimitive.If>
            <ThreadPrimitive.If running>
              <ComposerPrimitive.Cancel className="inline-flex h-11 shrink-0 items-center justify-center rounded-xl border px-4 text-sm font-medium transition-colors hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50">
                Stop
              </ComposerPrimitive.Cancel>
            </ThreadPrimitive.If>
          </ComposerPrimitive.Root>
        </div>
      </ThreadPrimitive.Root>
    </div>
  );
}

// ── Public export ─────────────────────────────────────────────────────────────

export function AssistantChat({ className }: { className?: string }) {
  return (
    <AssistantRuntimeBoundary>
      <AssistantChatPanel className={className} />
    </AssistantRuntimeBoundary>
  );
}
