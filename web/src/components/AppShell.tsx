import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { getHealthApiUrl } from "@/lib/env";
import {
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

  return (
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
  );
}
