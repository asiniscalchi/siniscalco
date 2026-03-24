import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { getHealthApiUrl } from "@/lib/api";
import {
  AlertCircleIcon,
  CheckCircleIcon,
  EyeClosedIcon,
  EyeIcon,
  LogoIcon,
  RefreshCwIcon,
} from "@/components/Icons";
import { useUiState } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

export function AppShell() {
  const { hideValues, toggleHideValues } = useUiState();
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
      <header className="sticky top-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-4 px-4 py-3 sm:gap-6 sm:px-6 sm:py-4">
          <div className="flex shrink-0 items-center gap-2">
            <div
              aria-label="Siniscalco"
              className="flex size-9 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-sm"
              role="img"
            >
              <LogoIcon className="size-5" />
            </div>
          </div>

          <nav aria-label="Primary" className="scrollbar-hide flex-1 overflow-x-auto">
            <div className="flex items-center gap-4 sm:gap-6">
              <NavLink
                className={({ isActive }) =>
                  cn(
                    "inline-flex items-center px-1 py-1 text-sm font-medium whitespace-nowrap transition-colors border-b-2",
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
                    "inline-flex items-center px-1 py-1 text-sm font-medium whitespace-nowrap transition-colors border-b-2",
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
                    "inline-flex items-center px-1 py-1 text-sm font-medium whitespace-nowrap transition-colors border-b-2",
                    isActive
                      ? "border-foreground text-foreground"
                      : "border-transparent text-muted-foreground hover:text-foreground",
                  )
                }
                to="/assets"
              >
                Assets
              </NavLink>
              <NavLink
                className={({ isActive }) =>
                  cn(
                    "inline-flex items-center px-1 py-1 text-sm font-medium whitespace-nowrap transition-colors border-b-2",
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

          <div className="flex shrink-0 items-center gap-2 sm:gap-3">
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
                "inline-flex size-9 items-center justify-center rounded-full border shadow-sm transition-colors",
                backendStatus === "connected" &&
                  "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900/30 dark:bg-emerald-900/20 dark:text-emerald-400",
                backendStatus === "checking" &&
                  "border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-900/30 dark:bg-amber-900/20 dark:text-amber-400",
                backendStatus === "unavailable" &&
                  "border-destructive/20 bg-destructive/10 text-destructive",
              )}
              title={`Backend: ${backendStatus}`}
            >
              {backendStatus === "connected" && <CheckCircleIcon className="size-4" />}
              {backendStatus === "checking" && (
                <RefreshCwIcon className="size-4 animate-spin" />
              )}
              {backendStatus === "unavailable" && <AlertCircleIcon className="size-4" />}
              <span className="sr-only">Backend {backendStatus}</span>
            </div>
          </div>
        </div>
      </header>
      <div className="mx-auto w-full max-w-6xl px-4 py-8 sm:px-6">
        <Outlet />
      </div>
    </div>
  );
}

