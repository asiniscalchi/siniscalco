import type { ReactNode } from "react";
import {
  AssistantRuntimeProvider,
  ComposerPrimitive,
  MessagePartPrimitive,
  MessagePrimitive,
  ThreadPrimitive,
  useLocalRuntime,
  type ChatModelAdapter,
  type ThreadMessageLike,
} from "@assistant-ui/react";

import { getAssistantChatApiUrl } from "@/lib/env";
import { cn } from "@/lib/utils";

function extractLatestUserText(messages: readonly ThreadMessageLike[]) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role !== "user") continue;

    if (typeof message.content === "string") {
      return message.content.trim();
    }

    if (!Array.isArray(message.content)) {
      return "";
    }

    return message.content
      .flatMap((part) => {
        if (part.type !== "text") return [];
        return typeof part.text === "string" ? [part.text] : [];
      })
      .join(" ")
      .trim();
  }

  return "";
}

type AssistantChatApiMessage = {
  role: string;
  content: string;
};

type AssistantChatApiResponse = {
  message: string;
};

type AssistantChatApiErrorResponse = {
  error?: string;
};

function serializeMessages(
  messages: readonly ThreadMessageLike[],
): AssistantChatApiMessage[] {
  return messages
    .map((message) => ({
      role: message.role,
      content: extractLatestUserText([message]),
    }))
    .filter((message) => message.content.length > 0);
}

async function requestAssistantReply(
  messages: readonly ThreadMessageLike[],
  abortSignal: AbortSignal,
) {
  const response = await fetch(getAssistantChatApiUrl(), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      messages: serializeMessages(messages),
    }),
    signal: abortSignal,
  });

  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as
      | AssistantChatApiErrorResponse
      | null;

    throw new Error(
      payload?.error || `assistant backend request failed with ${response.status}`,
    );
  }

  const payload = (await response.json()) as AssistantChatApiResponse;
  return payload.message;
}

const assistantModelAdapter: ChatModelAdapter = {
  async run({ messages, abortSignal }) {
    const reply = await requestAssistantReply(messages, abortSignal);

    return {
      content: [
        {
          type: "text",
          text: reply,
        },
      ],
    };
  },
};

function AssistantRuntimeBoundary({
  children,
}: Readonly<{ children: ReactNode }>) {
  const runtime = useLocalRuntime(assistantModelAdapter);

  return (
    <AssistantRuntimeProvider runtime={runtime}>
      {children}
    </AssistantRuntimeProvider>
  );
}

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

function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="flex flex-col items-start gap-2">
      <span className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
        Assistant
      </span>
      <div className="max-w-[85%] rounded-2xl rounded-bl-md border bg-background px-4 py-3 text-sm shadow-sm">
        <MessagePrimitive.Parts components={{ Text: MessageText }} />
      </div>
    </MessagePrimitive.Root>
  );
}

type AssistantChatProps = {
  className?: string;
};

export function AssistantChat({ className }: AssistantChatProps) {
  return (
    <AssistantRuntimeBoundary>
      <div className={cn("flex min-h-0 flex-1 flex-col", className)}>
        <ThreadPrimitive.Root className="flex min-h-0 flex-1 flex-col">
          <ThreadPrimitive.Viewport className="flex-1 space-y-6 overflow-y-auto bg-muted/20 px-4 py-5">
            <ThreadPrimitive.Empty>
              <div className="flex h-full min-h-[18rem] flex-col items-start justify-center gap-4">
                <div className="space-y-2">
                  <p className="text-sm font-medium uppercase tracking-[0.2em] text-muted-foreground">
                    Ready
                  </p>
                  <h3 className="text-2xl font-semibold tracking-tight">
                    Ask about the app
                  </h3>
                </div>
                <p className="max-w-xl text-sm leading-6 text-muted-foreground">
                  This starter chat is intentionally local-only. Try prompts
                  about the portfolio, accounts, assets, transactions, or how to
                  wire a real chat backend next.
                </p>
              </div>
            </ThreadPrimitive.Empty>

            <ThreadPrimitive.Messages
              components={{
                UserMessage,
                AssistantMessage,
              }}
            />
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
    </AssistantRuntimeBoundary>
  );
}
