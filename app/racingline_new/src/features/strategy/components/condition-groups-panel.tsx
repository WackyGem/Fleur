import { Fragment } from "react"

import { Button } from "@/components/ui/button"
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Separator } from "@/components/ui/separator"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { AddDashedButton } from "@/features/strategy/components/add-dashed-button"
import { ComparisonFields } from "@/features/strategy/components/comparison-fields"
import type {
  GroupLogic,
  IndicatorCatalog,
  StrategyCondition,
  StrategyConditionGroup,
} from "@/features/strategy/types"
import { Trash2 } from "lucide-react"

type ConditionGroupsPanelProps = {
  catalogOptions: IndicatorCatalog[]
  conditionGroups: StrategyConditionGroup[]
  onAddCondition: (groupId: string) => void
  onCreateGroup: () => void
  onRemoveCondition: (groupId: string, conditionId: string) => void
  onRemoveGroup: (groupId: string) => void
  onUpdateCondition: (
    groupId: string,
    conditionId: string,
    patch: Partial<StrategyCondition>
  ) => void
  onUpdateGroup: (
    groupId: string,
    patch: Partial<Pick<StrategyConditionGroup, "name">>
  ) => void
}

function ConditionGroupsPanel({
  catalogOptions,
  conditionGroups,
  onAddCondition,
  onCreateGroup,
  onRemoveCondition,
  onRemoveGroup,
  onUpdateCondition,
  onUpdateGroup,
}: ConditionGroupsPanelProps) {
  return (
    <FieldSet>
      <FieldLegend>指标组</FieldLegend>
      <div className="flex flex-col gap-3 pb-2">
        {conditionGroups.map((group, groupIndex) => (
          <Fragment key={group.id}>
            {groupIndex > 0 ? (
              <div className="px-1 text-center text-xs font-semibold text-muted-foreground">
                AND
              </div>
            ) : null}

            <div className="border border-border/60 bg-background">
              <Field
                orientation="horizontal"
                className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 bg-muted/15 px-3 py-2"
              >
                <FieldLabel className="shrink-0">组名</FieldLabel>
                <Input
                  className="h-7"
                  value={group.name}
                  onChange={(event) =>
                    onUpdateGroup(group.id, { name: event.target.value })
                  }
                />
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="text-muted-foreground hover:text-foreground"
                  onClick={() => onRemoveGroup(group.id)}
                  aria-label="删除指标组"
                  type="button"
                >
                  <Trash2 />
                </Button>
              </Field>

              <div className="flex flex-col gap-2 p-3 pt-2">
                {group.conditions.length === 0 ? (
                  <AddDashedButton
                    label="添加指标"
                    onClick={() => onAddCondition(group.id)}
                  />
                ) : (
                  <FieldGroup className="gap-2">
                    {group.conditions.map((condition, conditionIndex) => (
                      <Fragment key={condition.id}>
                        {conditionIndex > 0 ? (
                          <ConditionLogicToggle
                            logic={condition.logic}
                            onChange={(logic) =>
                              onUpdateCondition(group.id, condition.id, {
                                logic,
                              })
                            }
                          />
                        ) : null}

                        <ComparisonFields
                          catalogOptions={catalogOptions}
                          className="bg-muted/10 p-2 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_minmax(0,0.8fr)_auto_minmax(0,1fr)_minmax(0,1.1fr)_auto]"
                          value={condition}
                          onChange={(patch) =>
                            onUpdateCondition(group.id, condition.id, patch)
                          }
                          onRemove={() =>
                            onRemoveCondition(group.id, condition.id)
                          }
                          removeLabel="删除指标"
                        />
                      </Fragment>
                    ))}
                  </FieldGroup>
                )}

                <AddDashedButton
                  label="添加指标"
                  onClick={() => onAddCondition(group.id)}
                />
              </div>
            </div>
          </Fragment>
        ))}

        <AddDashedButton
          label="创建指标组"
          description="指标组之间固定按 AND 组合；组内每个指标可单独选择 AND 或 OR。"
          onClick={onCreateGroup}
          size="large"
        />
      </div>
    </FieldSet>
  )
}

function ConditionLogicToggle({
  logic,
  onChange,
}: {
  logic: GroupLogic
  onChange: (logic: GroupLogic) => void
}) {
  return (
    <div className="flex items-center gap-2">
      <Separator className="flex-1" />
      <ToggleGroup
        value={[logic]}
        onValueChange={(nextValue) => {
          const next = nextValue[0] as GroupLogic | undefined
          if (next) {
            onChange(next)
          }
        }}
        variant="outline"
        size="sm"
        spacing={0}
      >
        <ToggleGroupItem
          value="and"
          className="text-muted-foreground/60 aria-pressed:text-foreground/75"
        >
          AND
        </ToggleGroupItem>
        <ToggleGroupItem
          value="or"
          className="text-muted-foreground/60 aria-pressed:text-foreground/75"
        >
          OR
        </ToggleGroupItem>
      </ToggleGroup>
      <Separator className="flex-1" />
    </div>
  )
}

export { ConditionGroupsPanel }
