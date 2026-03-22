import { Outlet } from 'react-router-dom'

export function AppShell() {
  return (
    <div className="min-h-svh bg-muted/30">
      <header className="border-b bg-background/95">
        <div className="mx-auto flex w-full max-w-6xl px-6 py-4" />
      </header>
      <div className="mx-auto w-full max-w-6xl px-6 py-8">
        <Outlet />
      </div>
    </div>
  )
}
