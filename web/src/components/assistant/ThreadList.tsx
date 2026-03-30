import { useRef, useState } from "react";
import {
  ThreadListItemPrimitive,
  ThreadListItemMorePrimitive,
  ThreadListPrimitive,
  useAui,
} from "@assistant-ui/react";

import { PlusIcon, TrashIcon, PencilIcon } from "@/components/Icons";
import { cn } from "@/lib/utils";

function ThreadItemTitle() {
  const aui = useAui();
  const [renaming, setRenaming] = useState(false);
  const [draft, setDraft] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const title = aui.threadListItem().getState().title ?? "New chat";

  async function commitRename() {
    const trimmed = draft.trim();
    if (trimmed && trimmed !== title) {
      await aui.threadListItem().rename(trimmed);
    }
    setRenaming(false);
  }

  if (renaming) {
    return (
      <input
        ref={inputRef}
        autoFocus
        className="w-full bg-transparent text-sm outline-none"
        onBlur={() => void commitRename()}
        onChange={(e) => setDraft(e.target.value)}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Enter") void commitRename();
          if (e.key === "Escape") setRenaming(false);
        }}
        value={draft}
      />
    );
  }

  return (
    <span className="truncate text-sm text-foreground">
      <ThreadListItemPrimitive.Title fallback="New chat" />
    </span>
  );
}

function ThreadItem({ onSelect }: { onSelect?: () => void }) {
  const aui = useAui();

  return (
    <ThreadListItemPrimitive.Root className="group relative flex items-center rounded-lg transition-colors hover:bg-muted/60 data-[active=true]:bg-muted">
      <ThreadListItemPrimitive.Trigger
        className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 px-3 py-2.5 text-left"
        onClick={onSelect}
      >
        <ThreadItemTitle />
      </ThreadListItemPrimitive.Trigger>

      <ThreadListItemMorePrimitive.Root>
        <ThreadListItemMorePrimitive.Trigger
          className="mr-1 flex size-7 shrink-0 items-center justify-center rounded-md opacity-0 transition-opacity hover:bg-muted group-hover:opacity-100 data-[state=open]:opacity-100"
        >
          <span className="flex gap-0.5">
            <span className="size-1 rounded-full bg-muted-foreground" />
            <span className="size-1 rounded-full bg-muted-foreground" />
            <span className="size-1 rounded-full bg-muted-foreground" />
          </span>
          <span className="sr-only">Thread options</span>
        </ThreadListItemMorePrimitive.Trigger>

        <ThreadListItemMorePrimitive.Content className="z-50 min-w-[10rem] overflow-hidden rounded-lg border bg-popover p-1 shadow-lg">
          <ThreadListItemMorePrimitive.Item
            className="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 text-sm transition-colors hover:bg-muted"
            onSelect={() => {
              const title = aui.threadListItem().getState().title ?? "New chat";
              void aui.threadListItem().rename(
                window.prompt("Rename chat", title) ?? title,
              );
            }}
          >
            <PencilIcon className="size-3.5" />
            Rename
          </ThreadListItemMorePrimitive.Item>
          <ThreadListItemMorePrimitive.Separator className="my-1 h-px bg-border" />
          <ThreadListItemPrimitive.Delete asChild>
            <ThreadListItemMorePrimitive.Item className="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 text-sm text-destructive transition-colors hover:bg-destructive/10">
              <TrashIcon className="size-3.5" />
              Delete
            </ThreadListItemMorePrimitive.Item>
          </ThreadListItemPrimitive.Delete>
        </ThreadListItemMorePrimitive.Content>
      </ThreadListItemMorePrimitive.Root>
    </ThreadListItemPrimitive.Root>
  );
}

type ThreadListProps = {
  onSelect?: () => void;
  className?: string;
};

export function ThreadList({ onSelect, className }: ThreadListProps) {
  return (
    <ThreadListPrimitive.Root className={cn("flex flex-col", className)}>
      <div className="flex items-center justify-between px-1 pb-2">
        <span className="text-xs font-semibold uppercase tracking-[0.15em] text-muted-foreground">
          Chats
        </span>
        <ThreadListPrimitive.New asChild>
          <button
            className="flex size-7 items-center justify-center rounded-md transition-colors hover:bg-muted"
            title="New chat"
            type="button"
          >
            <PlusIcon className="size-3.5" />
            <span className="sr-only">New chat</span>
          </button>
        </ThreadListPrimitive.New>
      </div>

      <div className="flex-1 overflow-y-auto">
        <ThreadListPrimitive.Items
          components={{
            ThreadListItem: () => <ThreadItem onSelect={onSelect} />,
          }}
        />
      </div>
    </ThreadListPrimitive.Root>
  );
}
