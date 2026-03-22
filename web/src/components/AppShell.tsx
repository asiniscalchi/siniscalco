import { useEffect, useState } from 'react'
import { NavLink, Outlet } from 'react-router-dom'

import { getHealthApiUrl } from '@/lib/api'
import { cn } from '@/lib/utils'

export function AppShell() {
  const [backendStatus, setBackendStatus] = useState<
    'checking' | 'connected' | 'unavailable'
  >('checking')

  useEffect(() => {
    let cancelled = false

    async function checkBackendHealth() {
      setBackendStatus('checking')

      try {
        const response = await fetch(getHealthApiUrl())

        if (!cancelled) {
          setBackendStatus(response.status === 200 ? 'connected' : 'unavailable')
        }
      } catch {
        if (!cancelled) {
          setBackendStatus('unavailable')
        }
      }
    }

    void checkBackendHealth()

    return () => {
      cancelled = true
    }
  }, [])

  return (
    <div className="min-h-svh bg-muted/30">
      <header className="border-b bg-background/95">
        <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-6 px-6 py-4">
          <div className="space-y-1">
            <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
              Portfolio App
            </p>
            <p className="text-lg font-semibold tracking-tight">Siniscalco</p>
          </div>

          <nav aria-label="Primary">
            <NavLink
              className={({ isActive }) =>
                cn(
                  'inline-flex items-center rounded-full border px-4 py-2 text-sm font-medium transition-colors',
                  isActive
                    ? 'border-foreground bg-foreground text-background'
                    : 'border-border bg-background text-muted-foreground hover:text-foreground'
                )
              }
              to="/accounts"
            >
              Accounts
            </NavLink>
          </nav>

          <div
            aria-live="polite"
            className={cn(
              'inline-flex items-center rounded-full border px-3 py-1.5 text-sm font-medium capitalize',
              backendStatus === 'connected' &&
                'border-emerald-200 bg-emerald-50 text-emerald-700',
              backendStatus === 'checking' &&
                'border-amber-200 bg-amber-50 text-amber-700',
              backendStatus === 'unavailable' &&
                'border-destructive/20 bg-destructive/10 text-destructive'
            )}
          >
            {backendStatus}
          </div>
        </div>
      </header>
      <div className="mx-auto w-full max-w-6xl px-6 py-8">
        <Outlet />
      </div>
    </div>
  )
}
