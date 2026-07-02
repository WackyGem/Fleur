import { useEffect, useState, type ReactNode } from "react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { cn } from "@/lib/utils"

export const strategyToastAnimationMs = 180
export const strategyToastVisibleMs = 2_000
export const strategyToastLeaveDelayMs =
  strategyToastVisibleMs - strategyToastAnimationMs

type StrategyToastViewportProps = {
  children: ReactNode
}

function StrategyToastViewport({ children }: StrategyToastViewportProps) {
  return (
    <div className="pointer-events-none fixed top-[4.5rem] right-4 left-4 z-50 flex w-auto flex-col gap-2 sm:left-auto sm:w-96">
      {children}
    </div>
  )
}

type StrategyAutoDismissToastProps = {
  description: ReactNode
  title: ReactNode
}

function StrategyAutoDismissToast({
  description,
  title,
}: StrategyAutoDismissToastProps) {
  const [visible, setVisible] = useState(true)
  const [isLeaving, setIsLeaving] = useState(false)

  useEffect(() => {
    const leaveTimeoutId = window.setTimeout(
      () => setIsLeaving(true),
      strategyToastLeaveDelayMs
    )
    const removeTimeoutId = window.setTimeout(
      () => setVisible(false),
      strategyToastVisibleMs
    )

    return () => {
      window.clearTimeout(leaveTimeoutId)
      window.clearTimeout(removeTimeoutId)
    }
  }, [])

  if (!visible) {
    return null
  }

  return (
    <Alert
      className={cn(
        "pointer-events-auto shadow-lg",
        isLeaving ? "racingline-toast-leave" : "racingline-toast-enter"
      )}
    >
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>{description}</AlertDescription>
    </Alert>
  )
}

export { StrategyAutoDismissToast, StrategyToastViewport }
