import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Separator } from "@/components/ui/separator"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { AddDashedButton } from "@/features/strategy/components/add-dashed-button"
import { ComparisonFields } from "@/features/strategy/components/comparison-fields"
import { WeightScoreSlider } from "@/features/strategy/components/weight-score-slider"
import type {
  IndicatorCatalog,
  WeightIndicator,
} from "@/features/strategy/types"
import {
  clampScore,
  formatWeightIndicator,
  getScaledWeightIndicators,
} from "@/features/strategy/utils"

type WeightIndicatorsPanelProps = {
  catalogOptions: IndicatorCatalog[]
  onAddIndicator: () => void
  onRemoveIndicator: (indicatorId: string) => void
  onUpdateIndicator: (
    indicatorId: string,
    patch: Partial<WeightIndicator>
  ) => void
  weightIndicators: WeightIndicator[]
}

function WeightIndicatorsPanel({
  catalogOptions,
  onAddIndicator,
  onRemoveIndicator,
  onUpdateIndicator,
  weightIndicators,
}: WeightIndicatorsPanelProps) {
  return (
    <FieldSet className="h-full min-h-0 min-w-0">
      <FieldLegend>指标权重</FieldLegend>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 pb-2">
        <div className="min-h-0 min-w-0 flex-1 overflow-y-auto pr-1">
          {weightIndicators.length === 0 ? (
            <AddDashedButton label="新增指标权重" onClick={onAddIndicator} />
          ) : (
            <div className="flex min-w-0 flex-col gap-2">
              <FieldGroup className="min-w-0 gap-3">
                {weightIndicators.map((indicator) => {
                  const clampedScore = clampScore(indicator.score)

                  return (
                    <div
                      key={indicator.id}
                      className="flex min-w-0 flex-col gap-2 bg-muted/10 p-2"
                    >
                      <ComparisonFields
                        catalogOptions={catalogOptions}
                        className="lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_minmax(0,0.8fr)_auto_minmax(0,1fr)_minmax(0,1.1fr)_minmax(16rem,1.4fr)_5rem_auto]"
                        value={indicator}
                        onChange={(patch) =>
                          onUpdateIndicator(indicator.id, patch)
                        }
                        onRemove={() => onRemoveIndicator(indicator.id)}
                        removeLabel="删除权重指标"
                      >
                        <Field>
                          <FieldLabel>权重得分</FieldLabel>
                          <div className="flex h-10 items-center gap-3">
                            <WeightScoreSlider
                              value={clampedScore}
                              onValueChange={(nextValue) => {
                                onUpdateIndicator(indicator.id, {
                                  score: clampScore(nextValue),
                                })
                              }}
                            />
                          </div>
                        </Field>

                        <Field>
                          <FieldLabel>分数</FieldLabel>
                          <Input
                            className="text-center"
                            value={String(indicator.score)}
                            onChange={(event) =>
                              onUpdateIndicator(indicator.id, {
                                score: Number(event.target.value),
                              })
                            }
                            min={0}
                            max={100}
                            type="number"
                          />
                        </Field>
                      </ComparisonFields>
                    </div>
                  )
                })}
              </FieldGroup>

              <AddDashedButton
                className="bg-transparent"
                label="新增指标权重"
                onClick={onAddIndicator}
              />
            </div>
          )}
        </div>

        <Separator />
        <div className="min-w-0 shrink-0 bg-background">
          <WeightScaleSummary
            catalogOptions={catalogOptions}
            weightIndicators={weightIndicators}
          />
        </div>
      </div>
    </FieldSet>
  )
}

function WeightScaleSummary({
  catalogOptions,
  weightIndicators,
}: {
  catalogOptions: IndicatorCatalog[]
  weightIndicators: WeightIndicator[]
}) {
  const { indicators } = getScaledWeightIndicators(weightIndicators)

  return (
    <div className="min-w-0">
      <div className="mb-2 text-sm font-medium">权重得分</div>
      <Table className="w-full table-fixed sm:min-w-[32rem]">
        <TableHeader>
          <TableRow className="hover:bg-transparent">
            <TableHead>指标</TableHead>
            <TableHead className="w-24 text-left sm:w-36">缩放得分</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {indicators.length === 0 ? (
            <TableRow className="hover:bg-transparent">
              <TableCell className="truncate text-muted-foreground">
                暂无指标得分。
              </TableCell>
              <TableCell className="w-24 text-left text-muted-foreground tabular-nums sm:w-36">
                --
              </TableCell>
            </TableRow>
          ) : (
            indicators.map((indicator) => (
              <TableRow key={indicator.id} className="hover:bg-transparent">
                <TableCell className="truncate font-medium">
                  {formatWeightIndicator(indicator, { catalogOptions })}
                </TableCell>
                <TableCell className="w-24 text-left tabular-nums sm:w-36">
                  {indicator.scaledScore.toFixed(1)}
                </TableCell>
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
    </div>
  )
}

export { WeightIndicatorsPanel }
