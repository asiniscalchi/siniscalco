import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { getHealthApiUrl } from "@/lib/api";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

export function AppShell() {
  const { hideValues, toggleHideValues } = useUiState();
  const [backendStatus, setBackendStatus] = useState<
    "checking" | "connected" | "unavailable"
  >("checking");

  useEffect(() => {
    let cancelled = false;

    async function checkBackendHealth() {
      setBackendStatus("checking");

      try {
        const response = await fetch(getHealthApiUrl());

        if (!cancelled) {
          setBackendStatus(
            response.status === 200 ? "connected" : "unavailable",
          );
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

  return (
    <div className="min-h-svh bg-muted/30">
      <header className="border-b bg-background/95">
        <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-6 px-6 py-4">
          <div className="flex items-center gap-2">
            <p className="text-lg font-semibold tracking-tight">Siniscalco</p>
          </div>

          <nav aria-label="Primary">
            <div className="flex items-center gap-6">
              <NavLink
                className={({ isActive }) =>
                  cn(
                    "inline-flex items-center px-1 py-1 text-sm font-medium transition-colors border-b-2",
                    isActive
                      ? "border-foreground text-foreground"
                      : "border-transparent text-muted-foreground hover:text-foreground",
                  )
                }
                to="/portfolio"
              >
                Portfolio
              </NavLink>
              <NavLink
                className={({ isActive }) =>
                  cn(
                    "inline-flex items-center px-1 py-1 text-sm font-medium transition-colors border-b-2",
                    isActive
                      ? "border-foreground text-foreground"
                      : "border-transparent text-muted-foreground hover:text-foreground",
                  )
                }
                to="/accounts"
              >
                Accounts
              </NavLink>
              <NavLink
                className={({ isActive }) =>
                  cn(
                    "inline-flex items-center px-1 py-1 text-sm font-medium transition-colors border-b-2",
                    isActive
                      ? "border-foreground text-foreground"
                      : "border-transparent text-muted-foreground hover:text-foreground",
                  )
                }
                to="/transactions"
              >
                Transactions
              </NavLink>
            </div>
          </nav>

          <div className="flex items-center gap-3">
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
            <div
              aria-live="polite"
              className={cn(
                "inline-flex items-center rounded-full border px-3 py-1 text-xs font-medium capitalize",
                backendStatus === "connected" &&
                  "border-emerald-200 bg-emerald-50 text-emerald-700",
                backendStatus === "checking" &&
                  "border-amber-200 bg-amber-50 text-amber-700",
                backendStatus === "unavailable" &&
                  "border-destructive/20 bg-destructive/10 text-destructive",
              )}
            >
              {backendStatus}
            </div>
          </div>
        </div>
      </header>
      <div className="mx-auto w-full max-w-6xl px-6 py-8">
        <Outlet />
      </div>
    </div>
  );
}

function EyeIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-4"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      viewBox="0 0 24 24"
    >
      <path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6-10-6-10-6Z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

function EyeClosedIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-4"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      viewBox="0 0 24 24"
    >
      <path d="M3 3l18 18" />
      <path d="M10.6 10.6a2 2 0 0 0 2.8 2.8" />
      <path d="M9.4 5.1A11.4 11.4 0 0 1 12 4.8c6.5 0 10 7.2 10 7.2a17 17 0 0 1-4 4.6" />
      <path d="M6.6 6.7C4.1 8.4 2 12 2 12s3.5 7.2 10 7.2c1 0 1.9-.2 2.8-.5" />
    </svg>
  );
}
