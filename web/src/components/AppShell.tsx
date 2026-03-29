import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { NavLink, Outlet } from "react-router-dom";

import { AssistantChat } from "@/components/assistant";
import { Button } from "@/components/ui/button";
import { getHealthApiUrl } from "@/lib/env";
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

export function AppShell() {
  const { hideValues, toggleHideValues } = useUiState();
  const [assistantOpen, setAssistantOpen] = useState(false);
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

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setAssistantOpen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = "";
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [assistantOpen]);

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
                backendStatus === "unavailable" && "bg-destructive text-destructive-foreground",
              )}
              role="img"
              title={`Backend: ${backendStatus}`}
            >
              <LogoIcon className="size-5" />
              <span className="sr-only">Backend {backendStatus}</span>
            </div>
          </div>

          <nav aria-label="Primary" className="scrollbar-hide flex-1 overflow-x-auto">
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
              aria-label={hideValues ? "Show financial values" : "Hide financial values"}
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
              role="dialog"
            >
              <div className="flex h-[min(46rem,100%)] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border bg-background shadow-2xl">
                <div className="flex items-center justify-between border-b px-5 py-4">
                  <div className="space-y-1">
                    <h2 className="text-base font-semibold">Assistant</h2>
                    <p className="text-sm text-muted-foreground">
                      Popup chat entrypoint for quick questions inside the app.
                    </p>
                  </div>
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

                <AssistantChat className="min-h-0 flex-1" />
              </div>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
