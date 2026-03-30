import { useRef, useState } from "react";
import { ThreadListItemPrimitive, ThreadListPrimitive, useAui } from "@assistant-ui/react";

import { PlusIcon, TrashIcon, PencilIcon } from "@/components/Icons";
import { cn } from "@/lib/utils";

function ThreadItem({ onSelect }: { onSelect?: () => void }) {
  const aui = useAui();
  const [renaming, setRenaming] = useState(false);
  const [draft, setDraft] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const title = aui.threadListItem().getState().title ?? "New chat";

  function startRename(e: React.MouseEvent) {
    e.stopPropagation();
    setDraft(title);
    setRenaming(true);
    setTimeout(() => inputRef.current?.select(), 0);
  }

  async function commitRename() {
    const trimmed = draft.trim();
    if (trimmed && trimmed !== title) {
      await aui.threadListItem().rename(trimmed);
    }
    setRenaming(false);
  }

  return (
    <ThreadListItemPrimitive.Root className="group relative flex items-center rounded-lg transition-colors hover:bg-muted/60 data-[active=true]:bg-muted">
      <ThreadListItemPrimitive.Trigger
        className="flex min-w-0 flex-1 cursor-pointer items-center px-3 py-2.5 text-left"
        onClick={!renaming ? onSelect : undefined}
      >
        {renaming ? (
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
        ) : (
          <span className="truncate text-sm text-foreground">
            <ThreadListItemPrimitive.Title fallback="New chat" />
          </span>
        )}
      </ThreadListItemPrimitive.Trigger>

      {!renaming && (
        <div className="mr-1 flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
          <button
            className="flex size-6 items-center justify-center rounded-md transition-colors hover:bg-muted"
            onClick={startRename}
            title="Rename"
            type="button"
          >
            <PencilIcon className="size-3" />
            <span className="sr-only">Rename</span>
          </button>
          <ThreadListItemPrimitive.Delete asChild>
            <button
              className="flex size-6 items-center justify-center rounded-md text-destructive transition-colors hover:bg-destructive/10"
              title="Delete"
              type="button"
            >
              <TrashIcon className="size-3" />
              <span className="sr-only">Delete</span>
            </button>
          </ThreadListItemPrimitive.Delete>
        </div>
      )}
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
