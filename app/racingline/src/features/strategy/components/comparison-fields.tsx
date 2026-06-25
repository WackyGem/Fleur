import type { ReactNode } from "react"
import { Trash2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Field, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import type {
  ComparableIndicator,
  CompareTarget,
  ConditionOperator,
  IndicatorCatalog,
} from "@/features/strategy/types"
import {
  findCompatibleMetric,
  getCatalog,
  getCatalogMetricsByType,
  getComparableMetricPatch,
  getCompatibleOperator,
  getMetric,
  getOperatorLabel,
  getOperatorOptions,
} from "@/features/strategy/utils"
import { cn } from "@/lib/utils"

type ComparisonFieldsProps = {
  catalogOptions: IndicatorCatalog[]
  children?: ReactNode
  className?: string
  onChange: (patch: Partial<ComparableIndicator>) => void
  onRemove: () => void
  removeLabel: string
  value: ComparableIndicator
}

function ComparisonFields({
  catalogOptions,
  children,
  className,
  onChange,
  onRemove,
  removeLabel,
  value,
}: ComparisonFieldsProps) {
  if (catalogOptions.length === 0) {
    return null
  }

  const effectiveCatalogOptions = catalogOptions
  const catalog = getCatalog(value.catalogId, effectiveCatalogOptions)
  const metric = getMetric(
    value.catalogId,
    value.metric,
    effectiveCatalogOptions
  )
  const compareCatalog = getCatalog(
    value.compareCatalogId,
    effectiveCatalogOptions
  )
  const requireCrossing = isCrossingOperator(value.operator)
  const compareMetrics = getCatalogMetricsByType(
    value.compareCatalogId,
    metric.valueType,
    effectiveCatalogOptions,
    { requireCrossing }
  )
  const operatorChoices = getOperatorOptions(
    value.target,
    metric.valueType,
    metric.allowedOps
  )
  const compareTargetOptions = getCompareTargetOptions(
    value.operator,
    metric.valueType,
    metric.allowedOps
  )
  const valueLabel =
    value.operator === "between"
      ? "区间下限"
      : metric.valueType === "boolean"
        ? "布尔值"
        : "比较值"

  return (
    <div className={cn("grid gap-2 lg:items-end", className)}>
      <Field>
        <FieldLabel>指标类型</FieldLabel>
        <Select
          value={value.catalogId}
          onValueChange={(catalogId) => {
            if (!catalogId) {
              return
            }

            const nextCatalog = getCatalog(catalogId, effectiveCatalogOptions)
            const nextMetric = nextCatalog.metrics[0]
            onChange(
              getComparableMetricPatch(
                value,
                nextCatalog.id,
                nextMetric.id,
                effectiveCatalogOptions
              )
            )
          }}
        >
          <SelectTrigger className="h-10 w-full bg-background px-3">
            <SelectValue>
              <span className="truncate text-sm text-foreground">
                {catalog.label}
              </span>
            </SelectValue>
          </SelectTrigger>
          <SelectContent
            align="start"
            className="min-w-72 bg-background text-foreground"
          >
            <SelectGroup>
              <SelectLabel>指标来源</SelectLabel>
              {effectiveCatalogOptions.map((item) => (
                <SelectItem key={item.id} value={item.id}>
                  <span className="truncate text-xs font-medium">
                    {item.label}
                  </span>
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </Field>

      <Field>
        <FieldLabel>指标</FieldLabel>
        <Select
          value={value.metric}
          onValueChange={(metricId) => {
            if (metricId) {
              onChange(
                getComparableMetricPatch(
                  value,
                  value.catalogId,
                  metricId,
                  effectiveCatalogOptions
                )
              )
            }
          }}
        >
          <SelectTrigger className="h-10 w-full bg-background px-3">
            <SelectValue>
              <span className="truncate text-sm text-foreground">
                {metric.label}
              </span>
            </SelectValue>
          </SelectTrigger>
          <SelectContent
            align="start"
            className="min-w-72 bg-background text-foreground"
          >
            <SelectGroup>
              <SelectLabel>{catalog.source}</SelectLabel>
              {catalog.metrics.map((item) => (
                <SelectItem key={item.id} value={item.id}>
                  <span className="truncate text-xs">{item.label}</span>
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </Field>

      <Field>
        <FieldLabel>比较方式</FieldLabel>
        <Select
          value={value.operator}
          onValueChange={(operator) => {
            if (operator) {
              const nextOperator = operator as ConditionOperator
              const nextTargetOptions = getCompareTargetOptions(
                nextOperator,
                metric.valueType,
                metric.allowedOps
              )
              const nextTarget = nextTargetOptions.some(
                (target) => target === value.target
              )
                ? value.target
                : nextTargetOptions[0]
              const compatibleCompare = findCompatibleMetric(
                metric.valueType,
                value.compareCatalogId,
                value.compareMetric,
                effectiveCatalogOptions,
                { requireCrossing: isCrossingOperator(nextOperator) }
              )

              onChange({
                operator: nextOperator,
                target: nextTarget ?? "value",
                compareCatalogId: compatibleCompare.catalogId,
                compareMetric: compatibleCompare.metricId,
              })
            }
          }}
        >
          <SelectTrigger className="h-10 w-full bg-background px-3">
            <SelectValue>
              <span className="truncate text-sm text-foreground">
                {getOperatorLabel(value.operator)}
              </span>
            </SelectValue>
          </SelectTrigger>
          <SelectContent
            align="start"
            className="min-w-44 bg-background text-foreground"
          >
            <SelectGroup>
              <SelectLabel>操作符</SelectLabel>
              {operatorChoices.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  <span className="truncate text-xs">{option.label}</span>
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </Field>

      <Field>
        <FieldLabel>比较对象</FieldLabel>
        <Select
          value={value.target}
          onValueChange={(target) => {
            if (target) {
              const nextTarget = target as CompareTarget
              const nextOperator = getCompatibleOperator(
                value.operator,
                nextTarget,
                metric.valueType,
                metric.allowedOps
              )
              const compatibleCompare = findCompatibleMetric(
                metric.valueType,
                value.compareCatalogId,
                value.compareMetric,
                effectiveCatalogOptions,
                { requireCrossing: isCrossingOperator(nextOperator) }
              )
              onChange({
                target: nextTarget,
                operator: nextOperator,
                compareCatalogId: compatibleCompare.catalogId,
                compareMetric: compatibleCompare.metricId,
              })
            }
          }}
        >
          <SelectTrigger className="h-10 w-full bg-background px-3">
            <SelectValue>
              <span className="truncate text-sm text-foreground">
                {value.target === "value" ? "数值" : "指标"}
              </span>
            </SelectValue>
          </SelectTrigger>
          <SelectContent
            align="start"
            className="min-w-36 bg-background text-foreground"
          >
            <SelectGroup>
              <SelectLabel>比较对象</SelectLabel>
              {compareTargetOptions.map((target) => (
                <SelectItem key={target} value={target}>
                  <span className="truncate text-xs">
                    {target === "value" ? "数值" : "指标"}
                  </span>
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </Field>

      {value.operator === "is_null" ? null : value.target === "value" ? (
        <>
          <Field>
            <FieldLabel>{valueLabel}</FieldLabel>
            {metric.valueType === "boolean" ? (
              <Select
                value={value.value}
                onValueChange={(nextValue) => {
                  if (nextValue) {
                    onChange({ value: nextValue })
                  }
                }}
              >
                <SelectTrigger className="h-10 w-full bg-background px-3">
                  <SelectValue>
                    <span className="truncate text-sm text-foreground">
                      {value.value === "true" ? "true" : "false"}
                    </span>
                  </SelectValue>
                </SelectTrigger>
                <SelectContent
                  align="start"
                  className="min-w-36 bg-background text-foreground"
                >
                  <SelectGroup>
                    <SelectLabel>布尔值</SelectLabel>
                    <SelectItem value="true">
                      <span className="truncate text-xs">true</span>
                    </SelectItem>
                    <SelectItem value="false">
                      <span className="truncate text-xs">false</span>
                    </SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>
            ) : (
              <Input
                value={value.value}
                onChange={(event) => onChange({ value: event.target.value })}
                type={
                  metric.valueType === "date"
                    ? "date"
                    : metric.valueType === "number"
                      ? "number"
                      : "text"
                }
              />
            )}
          </Field>

          <Field
            data-disabled={
              metric.valueType !== "number" || value.operator !== "between"
                ? true
                : undefined
            }
          >
            <FieldLabel>区间上限</FieldLabel>
            <Input
              value={value.valueEnd}
              onChange={(event) => onChange({ valueEnd: event.target.value })}
              disabled={
                metric.valueType !== "number" || value.operator !== "between"
              }
              type="number"
            />
          </Field>
        </>
      ) : (
        <>
          <Field>
            <FieldLabel>对比类型</FieldLabel>
            <Select
              value={value.compareCatalogId}
              onValueChange={(catalogId) => {
                if (!catalogId) {
                  return
                }

                const nextCatalog = getCatalog(
                  catalogId,
                  effectiveCatalogOptions
                )
                const nextCompareMetric = getCatalogMetricsByType(
                  nextCatalog.id,
                  metric.valueType,
                  effectiveCatalogOptions,
                  { requireCrossing }
                )[0]

                if (nextCompareMetric) {
                  onChange({
                    compareCatalogId: nextCatalog.id,
                    compareMetric: nextCompareMetric.id,
                  })
                }
              }}
            >
              <SelectTrigger className="h-10 w-full bg-background px-3">
                <SelectValue>
                  <span className="truncate text-sm text-foreground">
                    {compareCatalog.label}
                  </span>
                </SelectValue>
              </SelectTrigger>
              <SelectContent
                align="start"
                className="min-w-72 bg-background text-foreground"
              >
                <SelectGroup>
                  <SelectLabel>指标来源</SelectLabel>
                  {effectiveCatalogOptions
                    .filter((item) =>
                      item.metrics.some(
                        (candidate) =>
                          candidate.valueType === metric.valueType &&
                          (!requireCrossing || candidate.supportsCrossing)
                      )
                    )
                    .map((item) => (
                      <SelectItem key={item.id} value={item.id}>
                        <span className="truncate text-xs font-medium">
                          {item.label}
                        </span>
                      </SelectItem>
                    ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>

          <Field>
            <FieldLabel>对比指标</FieldLabel>
            <Select
              value={value.compareMetric}
              onValueChange={(compareMetric) => {
                if (compareMetric) {
                  onChange({ compareMetric })
                }
              }}
            >
              <SelectTrigger className="h-10 w-full bg-background px-3">
                <SelectValue>
                  <span className="truncate text-sm text-foreground">
                    {compareMetrics.find(
                      (item) => item.id === value.compareMetric
                    )?.label ?? value.compareMetric}
                  </span>
                </SelectValue>
              </SelectTrigger>
              <SelectContent
                align="start"
                className="min-w-72 bg-background text-foreground"
              >
                <SelectGroup>
                  <SelectLabel>{compareCatalog.source}</SelectLabel>
                  {compareMetrics.map((compareMetric) => (
                    <SelectItem key={compareMetric.id} value={compareMetric.id}>
                      <span className="truncate text-xs">
                        {compareMetric.label}
                      </span>
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>

          {metric.valueType === "number" && !isCrossingOperator(value.operator) ? (
            <Field>
              <FieldLabel>对比倍数</FieldLabel>
              <Input
                value={value.compareMultiplier ?? "1"}
                onChange={(event) =>
                  onChange({ compareMultiplier: event.target.value })
                }
                type="number"
              />
            </Field>
          ) : null}
        </>
      )}

      {children}

      <Button
        variant="ghost"
        size="icon-sm"
        className="text-muted-foreground hover:text-foreground lg:[grid-column-start:-2] lg:justify-self-end"
        onClick={onRemove}
        aria-label={removeLabel}
        type="button"
      >
        <Trash2 />
      </Button>
    </div>
  )
}

function getCompareTargetOptions(
  operator: ConditionOperator,
  valueType: "number" | "boolean" | "string" | "date",
  allowedOps: ConditionOperator[]
): CompareTarget[] {
  return (["value", "metric"] as CompareTarget[]).filter((target) =>
    getOperatorOptions(target, valueType, allowedOps).some(
      (option) => option.value === operator
    )
  )
}

function isCrossingOperator(operator: ConditionOperator) {
  return operator === "crosses_above" || operator === "crosses_below"
}

export { ComparisonFields }
