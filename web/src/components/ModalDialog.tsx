import { createPortal } from "react-dom";

interface ModalDialogProps {
  title: string;
  description: string;
  size?: "md" | "2xl";
  children: React.ReactNode;
}

export function ModalDialog({ title, description, size = "2xl", children }: ModalDialogProps) {
  const panelClass =
    size === "md"
      ? "flex max-h-full w-full max-w-md flex-col overflow-hidden rounded-xl border bg-background shadow-2xl animate-in zoom-in-95 duration-200"
      : "my-auto flex max-h-full w-full max-w-2xl flex-col overflow-hidden rounded-xl border bg-background shadow-2xl animate-in zoom-in-95 duration-200";

  return createPortal(
    <div
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center overflow-y-auto bg-black/40 p-4 backdrop-blur-sm animate-in fade-in duration-200"
      role="dialog"
    >
      <div className={panelClass}>
        <header className="flex-none border-b px-6 py-4">
          <h2 className="text-lg font-semibold">{title}</h2>
          <p className="text-sm text-muted-foreground">{description}</p>
        </header>
        {children}
      </div>
    </div>,
    document.body,
  );
}
