import { ArrowLeft } from "lucide-react"

import { Button } from "@/components/ui/button"
import { strategySteps } from "@/features/strategy/catalog"
import type { Step } from "@/features/strategy/types"
import { cn } from "@/lib/utils"

type StrategyStepSidebarProps = {
  activeStep: Step
  onBack: () => void
  onStepChange: (step: Step) => void
}

function StrategyStepSidebar({
  activeStep,
  onBack,
  onStepChange,
}: StrategyStepSidebarProps) {
  return (
    <aside className="min-w-0 border-b border-border/70 pb-3 lg:border-r lg:border-b-0 lg:pr-5 lg:pb-4">
      <div className="mb-3 flex h-9 items-center gap-3 lg:mb-6">
        <Button
          variant="ghost"
          size="icon-sm"
          className="text-muted-foreground hover:bg-muted/60 hover:text-foreground"
          onClick={onBack}
          aria-label="返回看板"
          type="button"
        >
          <ArrowLeft />
        </Button>
        <h1 className="text-lg font-medium">选股</h1>
      </div>

      <ol className="grid min-w-0 grid-cols-5 gap-1 lg:flex lg:flex-col lg:gap-4">
        {strategySteps.map((step, index) => {
          const active = step.id === activeStep
          return (
            <li key={step.id} className="relative min-w-0 lg:w-full">
              {index > 0 ? (
                <span
                  className="absolute -top-4 left-[1.18rem] hidden h-4 w-px bg-border lg:block"
                  aria-hidden="true"
                />
              ) : null}
              <Button
                variant="ghost"
                className={cn(
                  "grid min-h-11 w-full min-w-0 grid-rows-[auto_auto] items-center justify-items-center gap-1 px-0.5 py-1.5 text-center hover:bg-muted/35 lg:h-9 lg:min-h-0 lg:grid-cols-[1.5rem_1fr] lg:grid-rows-1 lg:justify-items-stretch lg:gap-3 lg:px-3 lg:py-2 lg:text-left",
                  active
                    ? "bg-muted/35 text-foreground"
                    : "text-muted-foreground"
                )}
                aria-current={active ? "step" : undefined}
                type="button"
                onClick={() => onStepChange(step.id)}
              >
                <span
                  className={cn(
                    "size-1.5 justify-self-center lg:size-2",
                    active ? "bg-foreground" : "bg-border"
                  )}
                  aria-hidden="true"
                />
                <span className="w-full truncate text-[11px] leading-none font-medium sm:text-xs lg:text-sm lg:leading-normal">
                  {step.label}
                </span>
              </Button>
            </li>
          )
        })}
      </ol>
    </aside>
  )
}

export { StrategyStepSidebar }
