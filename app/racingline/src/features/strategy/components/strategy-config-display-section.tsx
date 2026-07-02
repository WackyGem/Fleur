import { cn } from "@/lib/utils"
import type { StrategyConfigDisplayModel } from "@/features/strategy/config-display"

export function StrategyConfigDisplaySection({
  className,
  display,
}: {
  className?: string
  display: StrategyConfigDisplayModel
}) {
  return (
    <div className={cn("flex flex-col gap-4", className)}>
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-medium">指标过滤</h3>
        <div className="flex flex-col divide-y divide-border/70 border-y border-border/70">
          {display.conditionRows.length > 0 ? (
            display.conditionRows.map((row) => (
              <div
                className="grid gap-1 py-2 md:grid-cols-[9rem_5rem_1fr]"
                key={row.id}
              >
                <div className="text-muted-foreground">{row.groupLabel}</div>
                <div>{row.logicLabel}</div>
                <div className="font-mono text-[11px] break-words">
                  {row.expression}
                </div>
              </div>
            ))
          ) : (
            <div className="py-3 text-muted-foreground">暂无条件指标</div>
          )}
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-medium">权重得分</h3>
        <div className="flex flex-col divide-y divide-border/70 border-y border-border/70">
          {display.scoringRows.length > 0 ? (
            display.scoringRows.map((row) => (
              <div
                className="grid gap-1 py-2 md:grid-cols-[4rem_5rem_1fr]"
                key={row.id}
              >
                <div className="text-muted-foreground">#{row.index}</div>
                <div className="font-medium">+{row.score}</div>
                <div className="font-mono text-[11px] break-words">
                  {row.expression}
                </div>
              </div>
            ))
          ) : (
            <div className="py-3 text-muted-foreground">暂无评分项</div>
          )}
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-medium">建仓摘要</h3>
        <div className="flex flex-col divide-y divide-border/70 border-y border-border/70">
          {display.buildSummaryRows.map((row) => (
            <div
              className="grid gap-1 py-2 md:grid-cols-[9rem_1fr]"
              key={row.label}
            >
              <div className="text-muted-foreground">{row.label}</div>
              <div>{row.value}</div>
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}
