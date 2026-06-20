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
    <aside className="border-b border-border/70 pb-4 lg:border-r lg:border-b-0 lg:pr-5">
      <div className="mb-6 flex h-9 items-center gap-3">
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

      <ol className="flex flex-col gap-4">
        {strategySteps.map((step, index) => {
          const active = step.id === activeStep
          return (
            <li key={step.id} className="relative">
              {index > 0 ? (
                <span
                  className="absolute -top-4 left-[1.18rem] h-4 w-px bg-border"
                  aria-hidden="true"
                />
              ) : null}
              <Button
                variant="ghost"
                className={cn(
                  "grid h-9 w-full grid-cols-[1.5rem_1fr] items-center gap-3 px-3 py-2 text-left hover:bg-muted/35",
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
                    "size-2 justify-self-center",
                    active ? "bg-foreground" : "bg-border"
                  )}
                  aria-hidden="true"
                />
                <span className="text-sm font-medium">{step.label}</span>
              </Button>
            </li>
          )
        })}
      </ol>
    </aside>
  )
}

export { StrategyStepSidebar }
