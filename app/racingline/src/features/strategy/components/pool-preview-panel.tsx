import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { StockPoolPreviewWorkbench } from "@/features/strategy/components/stock-pool-preview-workbench"
import type {
  IndicatorCatalog,
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import type { PreviewSnapshot } from "@/features/strategy/preview"

type PoolPreviewPanelProps = {
  appliedWeightIndicators: WeightIndicator[]
  catalogOptions: IndicatorCatalog[]
  conditionGroups: StrategyConditionGroup[]
  error?: string | null
  isPending?: boolean
  isStale?: boolean
  onUpdateWeightIndicator: (
    indicatorId: string,
    patch: Partial<WeightIndicator>
  ) => void
  previewSnapshot: PreviewSnapshot | null
  weightIndicators: WeightIndicator[]
}

function PoolPreviewPanel({
  appliedWeightIndicators,
  catalogOptions,
  conditionGroups,
  error,
  isPending,
  isStale,
  onUpdateWeightIndicator,
  previewSnapshot,
  weightIndicators,
}: PoolPreviewPanelProps) {
  const hasStrategyInput =
    conditionGroups.some((group) => group.conditions.length > 0) ||
    weightIndicators.length > 0

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      {error ? (
        <Alert variant="destructive" className="shrink-0">
          <AlertTitle>股池预览失败</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : isStale ? (
        <Alert className="shrink-0">
          <AlertTitle>股池预览已过期</AlertTitle>
          <AlertDescription>请更新股池后继续使用当前结果。</AlertDescription>
        </Alert>
      ) : isPending ? (
        <Alert className="shrink-0">
          <AlertTitle>股池预览执行中</AlertTitle>
          <AlertDescription>Rearview 正在执行当前规则。</AlertDescription>
        </Alert>
      ) : null}

      {previewSnapshot &&
      (previewSnapshot.timeline?.trade_dates.length ??
        previewSnapshot.result.trade_dates.length) === 0 ? (
        <Alert className="shrink-0">
          <AlertTitle>股池为空</AlertTitle>
          <AlertDescription>当前区间没有返回候选股票。</AlertDescription>
        </Alert>
      ) : null}

      <div className="min-h-0 flex-1">
        <StockPoolPreviewWorkbench
          appliedWeightIndicators={appliedWeightIndicators}
          catalogOptions={catalogOptions}
          conditionGroups={conditionGroups}
          hasStrategyInput={hasStrategyInput}
          onUpdateWeightIndicator={onUpdateWeightIndicator}
          previewSnapshot={previewSnapshot}
          weightIndicators={weightIndicators}
        />
      </div>
    </div>
  )
}

export { PoolPreviewPanel }
