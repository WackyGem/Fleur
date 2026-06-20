import { StockPoolPreviewWorkbench } from "@/features/strategy/components/stock-pool-preview-workbench"
import type {
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"

type PoolPreviewPanelProps = {
  appliedWeightIndicators: WeightIndicator[]
  conditionGroups: StrategyConditionGroup[]
  draftWeightIndicators: WeightIndicator[]
  onDraftWeightScoreChange: (indicatorId: string, score: number) => void
  weightIndicators: WeightIndicator[]
}

function PoolPreviewPanel({
  appliedWeightIndicators,
  conditionGroups,
  draftWeightIndicators,
  onDraftWeightScoreChange,
  weightIndicators,
}: PoolPreviewPanelProps) {
  const hasStrategyInput =
    conditionGroups.some((group) => group.conditions.length > 0) ||
    weightIndicators.length > 0

  return (
    <div className="h-full min-h-0">
      <StockPoolPreviewWorkbench
        appliedWeightIndicators={appliedWeightIndicators}
        conditionGroups={conditionGroups}
        draftWeightIndicators={draftWeightIndicators}
        hasStrategyInput={hasStrategyInput}
        onDraftWeightScoreChange={onDraftWeightScoreChange}
      />
    </div>
  )
}

export { PoolPreviewPanel }
