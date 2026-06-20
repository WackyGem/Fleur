import { Outlet } from "react-router-dom"

export function AppShell() {
  return (
    <div className="min-h-svh bg-background text-foreground">
      <header className="sticky top-0 z-10 border-b border-border/70 bg-background/95 backdrop-blur">
        <div className="flex h-14 items-center justify-between px-4 sm:px-6">
          <div className="flex min-w-0 items-center gap-4">
            <div className="flex h-8 min-w-0 items-center">
              <div className="text-sm leading-none font-medium">Racingline</div>
            </div>
          </div>
        </div>
      </header>

      <main className="px-4 py-4 sm:px-6">
        <Outlet />
      </main>
    </div>
  )
}
