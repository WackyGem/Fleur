import {
  ActivityCircleIcon,
  ChartLineData01Icon,
  DatabaseSyncIcon,
  DashboardSquare03Icon,
  Menu01Icon,
} from "@hugeicons/core-free-icons"
import { NavLink, Outlet } from "react-router-dom"

import { apiBaseUrl } from "@/api/client"
import { useHealthQuery } from "@/api/hooks"
import { RacinglineIcon } from "@/components/racingline/icon"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { buttonVariants } from "@/components/ui/button"
import { Separator } from "@/components/ui/separator"
import { cn } from "@/lib/utils"

const navItems = [
  { to: "/runs", label: "Runs", icon: DashboardSquare03Icon },
  { to: "/portfolios", label: "Portfolios", icon: ChartLineData01Icon },
  { to: "/rules", label: "Rules", icon: ActivityCircleIcon },
  { to: "/metrics", label: "Metrics", icon: ChartLineData01Icon },
]

export function AppShell() {
  const healthQuery = useHealthQuery()
  const healthOk = healthQuery.data?.status === "ok"

  return (
    <div className="min-h-dvh bg-background text-foreground">
      <header className="sticky top-0 border-b bg-background/95 backdrop-blur">
        <div className="mx-auto flex min-h-14 w-full max-w-7xl flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
              <RacinglineIcon icon={DatabaseSyncIcon} />
            </div>
            <div className="min-w-0">
              <div className="truncate text-sm font-medium">Racingline</div>
              <div className="truncate text-xs text-muted-foreground">
                {apiBaseUrl()}
              </div>
            </div>
          </div>
          <nav className="flex min-w-0 items-center gap-2 overflow-x-auto">
            {navItems.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  cn(
                    buttonVariants({
                      size: "sm",
                      variant: isActive ? "default" : "ghost",
                    })
                  )
                }
              >
                <RacinglineIcon icon={item.icon} inline="start" />
                {item.label}
              </NavLink>
            ))}
          </nav>
          <div className="hidden items-center gap-2 lg:flex">
            <RacinglineIcon icon={Menu01Icon} />
            <Badge variant={healthOk ? "secondary" : "destructive"}>
              {healthQuery.isPending
                ? "checking"
                : healthOk
                  ? "rearview ok"
                  : "rearview down"}
            </Badge>
          </div>
        </div>
      </header>

      <main className="mx-auto flex w-full max-w-7xl flex-col gap-4 px-3 py-4">
        {!healthOk && !healthQuery.isPending ? (
          <Alert variant="destructive">
            <RacinglineIcon icon={DatabaseSyncIcon} />
            <AlertTitle>Rearview API unavailable</AlertTitle>
            <AlertDescription>
              The workbench shell and local routes remain available; API-backed
              sections show explicit backend errors until health recovers.
            </AlertDescription>
          </Alert>
        ) : null}
        <Outlet />
      </main>
      <Separator />
    </div>
  )
}
