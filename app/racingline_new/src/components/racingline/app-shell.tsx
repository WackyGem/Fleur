import { NavLink, Outlet } from "react-router-dom"
import { HugeiconsIcon } from "@hugeicons/react"
import {
  ChartLineData01Icon,
  DashboardSpeedIcon,
} from "@hugeicons/core-free-icons"

import { Separator } from "@/components/ui/separator"
import { cn } from "@/lib/utils"

const navigation = [
  {
    href: "/dashboard",
    label: "看板",
    icon: DashboardSpeedIcon,
  },
  {
    href: "/strategies",
    label: "选股",
    icon: ChartLineData01Icon,
  },
]

function ShellNavItem({
  href,
  label,
  icon: Icon,
}: {
  href: string
  label: string
  icon: typeof DashboardSpeedIcon
}) {
  return (
    <NavLink
      to={href}
      className={({ isActive }) =>
        cn(
          "inline-flex h-8 items-center gap-1.5 border px-2.5 text-xs font-medium transition-colors",
          isActive
            ? "border-border bg-background text-foreground"
            : "border-transparent text-muted-foreground hover:bg-muted hover:text-foreground"
        )
      }
    >
      <HugeiconsIcon icon={Icon} data-icon="inline-start" />
      {label}
    </NavLink>
  )
}

export function AppShell() {
  return (
    <div className="min-h-svh bg-background text-foreground">
      <header className="sticky top-0 z-10 border-b border-border/70 bg-background/95 backdrop-blur">
        <div className="flex h-14 items-center justify-between px-4 sm:px-6">
          <div className="flex min-w-0 items-center gap-4">
            <div className="min-w-0">
              <div className="text-sm font-medium leading-none">Racingline</div>
            </div>
            <Separator orientation="vertical" className="h-6" />
            <nav className="flex items-center gap-1.5">
              {navigation.map((item) => (
                <ShellNavItem key={item.href} {...item} />
              ))}
            </nav>
          </div>
        </div>
      </header>

      <main className="px-4 py-4 sm:px-6">
        <Outlet />
      </main>
    </div>
  )
}
