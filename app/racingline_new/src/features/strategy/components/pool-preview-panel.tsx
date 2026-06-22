import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { StockPoolPreviewWorkbench } from "@/features/strategy/components/stock-pool-preview-workbench"
import type {
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import type {
  PreviewRange,
  PreviewSnapshot,
} from "@/features/strategy/preview"

type PoolPreviewPanelProps = {
  appliedWeightIndicators: WeightIndicator[]
  conditionGroups: StrategyConditionGroup[]
  draftWeightIndicators: WeightIndicator[]
  error?: string | null
  isPending?: boolean
  isStale?: boolean
  onDraftWeightScoreChange: (indicatorId: string, score: number) => void
  onPreviewRangeChange: (patch: Partial<PreviewRange>) => void
  previewRange: PreviewRange
  previewSnapshot: PreviewSnapshot | null
  weightIndicators: WeightIndicator[]
}

function PoolPreviewPanel({
  appliedWeightIndicators,
  conditionGroups,
  draftWeightIndicators,
  error,
  isPending,
  isStale,
  onDraftWeightScoreChange,
  onPreviewRangeChange,
  previewRange,
  previewSnapshot,
  weightIndicators,
}: PoolPreviewPanelProps) {
  const hasStrategyInput =
    conditionGroups.some((group) => group.conditions.length > 0) ||
    weightIndicators.length > 0

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <FieldGroup className="grid shrink-0 gap-3 md:grid-cols-3">
        <Field>
          <FieldLabel>开始日期</FieldLabel>
          <Input
            value={previewRange.startDate}
            onChange={(event) =>
              onPreviewRangeChange({ startDate: event.target.value })
            }
            type="date"
          />
        </Field>
        <Field>
          <FieldLabel>结束日期</FieldLabel>
          <Input
            value={previewRange.endDate}
            onChange={(event) =>
              onPreviewRangeChange({ endDate: event.target.value })
            }
            type="date"
          />
        </Field>
        <Field>
          <FieldLabel>展示行数</FieldLabel>
          <Input
            min={1}
            value={String(previewRange.previewRowLimit)}
            onChange={(event) =>
              onPreviewRangeChange({
                previewRowLimit: Number(event.target.value),
              })
            }
            type="number"
          />
        </Field>
      </FieldGroup>

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

      {previewSnapshot && previewSnapshot.result.trade_dates.length === 0 ? (
        <Alert className="shrink-0">
          <AlertTitle>股池为空</AlertTitle>
          <AlertDescription>当前区间没有返回候选股票。</AlertDescription>
        </Alert>
      ) : null}

      <div className="min-h-0 flex-1">
      <StockPoolPreviewWorkbench
        appliedWeightIndicators={appliedWeightIndicators}
        conditionGroups={conditionGroups}
        draftWeightIndicators={draftWeightIndicators}
        hasStrategyInput={hasStrategyInput}
        onDraftWeightScoreChange={onDraftWeightScoreChange}
        previewSnapshot={previewSnapshot}
      />
      </div>
    </div>
  )
}

export { PoolPreviewPanel }
