import { useMemo } from "react";
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

type AssistantChatApiMessage = { role: string; content: string };
type AssistantChatApiErrorResponse = { error?: string };

type ChatStreamEvent =
  | { type: "tool_call"; id: string; name: string; args: Record<string, unknown> }
  | { type: "tool_result"; id: string; result: unknown }
  | { type: "text"; text: string; model: string }
  | { type: "error"; error: string };

function extractMessageText(message: ThreadMessageLike): string {
  if (typeof message.content === "string") return message.content.trim();
  if (!Array.isArray(message.content)) return "";
  return message.content
    .flatMap((part) => (part.type !== "text" ? [] : typeof part.text === "string" ? [part.text] : []))
    .join(" ")
    .trim();
}

function serializeMessages(messages: readonly ThreadMessageLike[]): AssistantChatApiMessage[] {
  return messages
    .map((m) => ({ role: m.role, content: extractMessageText(m) }))
    .filter((m) => m.content.length > 0);
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

    const reader = response.body!.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    const toolCalls: ToolCallMessagePart[] = [];

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";

      for (const line of lines) {
        if (!line.startsWith("data: ")) continue;
        const event = JSON.parse(line.slice(6)) as ChatStreamEvent;

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
        } else if (event.type === "text") {
          yield { content: [...toolCalls, { type: "text" as const, text: event.text }] };
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

function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="flex flex-col items-start gap-2">
      <span className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
        Assistant
      </span>
      <div className="max-w-[85%] rounded-2xl rounded-bl-md border bg-background px-4 py-3 text-sm shadow-sm">
        <MessagePrimitive.Parts
          components={{ Text: AssistantMessageText, tools: { Fallback: ToolCallDisplay } }}
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
