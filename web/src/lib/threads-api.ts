import type { ExportedMessageRepository, ExportedMessageRepositoryItem } from "@assistant-ui/core";

import {
  getAssistantThreadApiUrl,
  getAssistantThreadMessagesApiUrl,
  getAssistantThreadsApiUrl,
} from "./env";

// ── Backend response types ────────────────────────────────────────────────────

export type ThreadMetadata = {
  id: string;
  title: string | null;
  status: "regular" | "archived";
  created_at: string;
  updated_at: string;
};

type MessagesResponse = {
  messages: Array<{
    id: string;
    parent_id: string | null;
    content: unknown;
    run_config: unknown | null;
  }>;
};

// ── Thread CRUD ───────────────────────────────────────────────────────────────

export async function apiListThreads(): Promise<ThreadMetadata[]> {
  const res = await fetch(getAssistantThreadsApiUrl());
  if (!res.ok) throw new Error(`list threads failed: ${res.status}`);
  return res.json() as Promise<ThreadMetadata[]>;
}

export async function apiCreateThread(id: string): Promise<ThreadMetadata> {
  const res = await fetch(getAssistantThreadsApiUrl(), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id }),
  });
  if (!res.ok) throw new Error(`create thread failed: ${res.status}`);
  return res.json() as Promise<ThreadMetadata>;
}

export async function apiGetThread(threadId: string): Promise<ThreadMetadata> {
  const res = await fetch(getAssistantThreadApiUrl(threadId));
  if (!res.ok) throw new Error(`get thread failed: ${res.status}`);
  return res.json() as Promise<ThreadMetadata>;
}

export async function apiRenameThread(threadId: string, title: string): Promise<void> {
  const res = await fetch(`${getAssistantThreadApiUrl(threadId)}/title`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ title }),
  });
  if (!res.ok) throw new Error(`rename thread failed: ${res.status}`);
}

export async function apiUpdateThreadStatus(
  threadId: string,
  status: "regular" | "archived",
): Promise<void> {
  const res = await fetch(`${getAssistantThreadApiUrl(threadId)}/status`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ status }),
  });
  if (!res.ok) throw new Error(`update thread status failed: ${res.status}`);
}

export async function apiDeleteThread(threadId: string): Promise<void> {
  const res = await fetch(getAssistantThreadApiUrl(threadId), { method: "DELETE" });
  if (!res.ok) throw new Error(`delete thread failed: ${res.status}`);
}

// ── Messages ──────────────────────────────────────────────────────────────────

export async function apiLoadMessages(threadId: string): Promise<ExportedMessageRepository> {
  const res = await fetch(getAssistantThreadMessagesApiUrl(threadId));
  if (!res.ok) throw new Error(`load messages failed: ${res.status}`);
  const body = (await res.json()) as MessagesResponse;

  // Deserialize createdAt string back to Date for each message
  const messages = body.messages.map((m) => {
    const msg = m.content as { createdAt?: string | Date };
    const message =
      typeof msg.createdAt === "string"
        ? ({ ...msg, createdAt: new Date(msg.createdAt) } as ExportedMessageRepositoryItem["message"])
        : (msg as ExportedMessageRepositoryItem["message"]);
    return {
      parentId: m.parent_id,
      message,
      runConfig: (m.run_config ?? undefined) as ExportedMessageRepositoryItem["runConfig"],
    };
  });

  return { messages };
}

export async function apiAppendMessage(
  threadId: string,
  item: ExportedMessageRepositoryItem,
): Promise<void> {
  const res = await fetch(getAssistantThreadMessagesApiUrl(threadId), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      id: item.message.id,
      parent_id: item.parentId,
      content: item.message,
      run_config: item.runConfig ?? null,
    }),
  });
  if (!res.ok) throw new Error(`append message failed: ${res.status}`);
}
