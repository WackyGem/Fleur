import type { ComponentProps, ReactNode } from "react"

import { Separator } from "@/components/ui/separator"
import { strategySplitPanelColumnsClassName } from "@/features/strategy/components/strategy-split-layout"
import { cn } from "@/lib/utils"

type StrategySplitPanelProps = Omit<ComponentProps<"div">, "children"> & {
  aside: ReactNode
  asideClassName?: string
  main: ReactNode
  mainClassName?: string
  mobileSeparatorClassName?: string
}

function StrategySplitPanel({
  aside,
  asideClassName,
  className,
  main,
  mainClassName,
  mobileSeparatorClassName,
  ...props
}: StrategySplitPanelProps) {
  return (
    <div
      className={cn(
        "grid min-h-full gap-y-4",
        strategySplitPanelColumnsClassName,
        "xl:gap-x-0",
        className
      )}
      {...props}
    >
      <div className={cn("flex min-h-0 flex-col gap-4 pt-5", mainClassName)}>
        {main}
      </div>

      <Separator className={cn("xl:hidden", mobileSeparatorClassName)} />
      <Separator className="hidden xl:block" orientation="vertical" />

      <div className={cn("flex min-h-0 flex-col gap-4 pt-5", asideClassName)}>
        {aside}
      </div>
    </div>
  )
}

export { StrategySplitPanel }
