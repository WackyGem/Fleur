import type {
  ConditionOperator,
  IndicatorCatalog,
  MetricOption,
  MetricValueType,
  StrategyCondition,
  StrategyConditionGroup,
} from "@/features/strategy/types"
import type {
  FilterExpr,
  MetricDefinition,
  MetricValueKind,
  Operand,
  Operator,
  RuleVersionSpec,
} from "@/types/rearview"

type BuildRuleSpecOptions = {
  topN?: number
}

export type ConditionFilterPath = {
  conditionId: string
  groupId: string
  path: string
}

export type BuildRuleSpecResult = {
  conditionPaths: ConditionFilterPath[]
  rule: RuleVersionSpec
}

type MetricIndex = Map<string, MetricDefinition>

const metricGroupLabels: Record<string, string> = {
  momentum: "动量指标",
  pattern: "形态特征",
  quotes: "行情与涨跌",
  trend: "趋势指标",
  trend_previous: "趋势前值",
  volume: "量能指标",
}

const martTableLabels: Record<string, string> = {
  mart_stock_momentum_indicator_daily: "动量指标",
  mart_stock_price_pattern_daily: "形态特征",
  mart_stock_quotes_daily: "行情与涨跌",
  mart_stock_trend_indicator_daily: "趋势指标",
  mart_stock_volume_indicator_daily: "量能指标",
}

const metricGroupOrder: Record<string, number> = {
  quotes: 10,
  trend: 20,
  momentum: 30,
  volume: 40,
  pattern: 50,
  trend_previous: 90,
}

type BuiltFilterNode =
  | {
      conditionId: string
      expr: FilterExpr
      groupId: string
      type: "leaf"
    }
  | {
      children: BuiltFilterNode[]
      type: "all" | "any"
    }

export class StrategyRuleSpecError extends Error {
  readonly conditionId?: string
  readonly groupId?: string

  constructor(
    message: string,
    options: { conditionId?: string; groupId?: string } = {}
  ) {
    super(message)
    this.name = "StrategyRuleSpecError"
    this.conditionId = options.conditionId
    this.groupId = options.groupId
  }
}

export function buildStrategyMetricCatalog(
  metrics: MetricDefinition[]
): IndicatorCatalog[] {
  const groups = new Map<string, IndicatorCatalog>()

  for (const metric of metrics) {
    if (!metric.allow_filter) {
      continue
    }

    const option = toMetricOption(metric)
    if (option.allowedOps.length === 0) {
      continue
    }

    const groupId = metric.display?.group ?? metric.mart_table
    const group =
      groups.get(groupId) ??
      createCatalogGroup(groupId, metric.display?.group, metric.mart_table)
    group.metrics.push(option)
    groups.set(groupId, group)
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      metrics: group.metrics.sort(compareMetricOptions),
    }))
    .sort(compareCatalogGroups)
}

export function buildStrategySelectionRuleSpec(
  conditionGroups: StrategyConditionGroup[],
  catalog: MetricDefinition[],
  options: BuildRuleSpecOptions = {}
): BuildRuleSpecResult {
  const metricIndex = buildMetricIndex(catalog)
  const groupNodes = conditionGroups.map((group) =>
    buildGroupFilterNode(group, metricIndex)
  )

  if (groupNodes.length === 0) {
    throw new StrategyRuleSpecError("至少需要一个指标组")
  }

  const conditionPaths: ConditionFilterPath[] = []
  const poolFilters = toFilterExpr(
    {
      children: groupNodes,
      type: "all",
    },
    "pool_filters",
    conditionPaths
  )

  return {
    conditionPaths,
    rule: {
      universe: {
        base: "all_a_shares",
        exclude_st: true,
        exclude_suspend: true,
        include_security_codes: [],
        exclude_security_codes: [],
      },
      pool_filters: poolFilters,
      scoring: {
        rules: [],
        clamp: {
          min: 0,
          max: 99,
        },
      },
      top_n_default: normalizeTopN(options.topN),
      output_metrics: buildOutputMetrics(conditionGroups, metricIndex),
    },
  }
}

export function buildGroupFilterExpr(
  group: StrategyConditionGroup,
  catalog: MetricDefinition[]
): FilterExpr {
  const conditionPaths: ConditionFilterPath[] = []
  return toFilterExpr(
    buildGroupFilterNode(group, buildMetricIndex(catalog)),
    "pool_filters.conditions.0",
    conditionPaths
  )
}

export function buildMixedLogicFilterExpr(
  conditions: StrategyCondition[],
  catalog: MetricDefinition[]
): FilterExpr {
  const conditionPaths: ConditionFilterPath[] = []
  return toFilterExpr(
    buildMixedLogicFilterNode("group", conditions, buildMetricIndex(catalog)),
    "pool_filters.conditions.0",
    conditionPaths
  )
}

function createCatalogGroup(
  id: string,
  displayGroup: string | null | undefined,
  martTable: string
): IndicatorCatalog {
  return {
    id,
    label: resolveMetricGroupLabel(id, displayGroup, martTable),
    source: martTable,
    metrics: [],
  }
}

function resolveMetricGroupLabel(
  id: string,
  displayGroup: string | null | undefined,
  martTable: string
) {
  return (
    metricGroupLabels[id] ??
    (displayGroup ? metricGroupLabels[displayGroup] : undefined) ??
    martTableLabels[martTable] ??
    displayGroup ??
    martTable
  )
}

function toMetricOption(metric: MetricDefinition): MetricOption {
  const valueType = mapMetricValueKind(metric.value_kind)
  const allowedOps = metric.allowed_ops.filter((operator) =>
    isSupportedUiOperator(
      operator,
      valueType,
      Boolean(metric.cross?.previous_metric)
    )
  )

  return {
    allowedOps,
    defaultOutput: metric.default_output,
    description: metric.description,
    id: metric.logical_metric,
    label: metric.display?.label_zh ?? metric.logical_metric,
    previousMetric: metric.cross?.previous_metric,
    sourceMetric: metric,
    supportsCrossing: Boolean(metric.cross?.previous_metric),
    valueType,
  }
}

function mapMetricValueKind(valueKind: MetricValueKind): MetricValueType {
  if (valueKind === "numeric" || valueKind === "integer") {
    return "number"
  }

  return valueKind
}

function isSupportedUiOperator(
  operator: Operator,
  valueType: MetricValueType,
  supportsCrossing: boolean
) {
  if (operator === "crosses_above" || operator === "crosses_below") {
    return valueType === "number" && supportsCrossing
  }

  if (operator === "between") {
    return valueType === "number"
  }

  if (
    operator === "gt" ||
    operator === "gte" ||
    operator === "lt" ||
    operator === "lte"
  ) {
    return valueType === "number"
  }

  if (operator === "eq" || operator === "ne" || operator === "is_null") {
    return valueType !== "date" || operator === "is_null"
  }

  return false
}

function compareCatalogGroups(left: IndicatorCatalog, right: IndicatorCatalog) {
  const leftOrder = groupSortOrder(left)
  const rightOrder = groupSortOrder(right)

  return leftOrder - rightOrder || left.label.localeCompare(right.label)
}

function groupSortOrder(group: IndicatorCatalog) {
  const configuredOrder = metricGroupOrder[group.id]
  if (configuredOrder !== undefined) {
    return configuredOrder
  }

  const firstMetric = group.metrics[0]?.sourceMetric?.display?.sort_order
  return firstMetric ?? Number.MAX_SAFE_INTEGER
}

function compareMetricOptions(left: MetricOption, right: MetricOption) {
  const leftOrder = left.sourceMetric?.display?.sort_order
  const rightOrder = right.sourceMetric?.display?.sort_order

  return (
    (leftOrder ?? Number.MAX_SAFE_INTEGER) -
      (rightOrder ?? Number.MAX_SAFE_INTEGER) ||
    left.label.localeCompare(right.label) ||
    left.id.localeCompare(right.id)
  )
}

function buildMetricIndex(catalog: MetricDefinition[]) {
  return new Map(catalog.map((metric) => [metric.logical_metric, metric]))
}

function buildGroupFilterNode(
  group: StrategyConditionGroup,
  metricIndex: MetricIndex
): BuiltFilterNode {
  if (group.conditions.length === 0) {
    throw new StrategyRuleSpecError("指标组不能为空", { groupId: group.id })
  }

  return buildMixedLogicFilterNode(group.id, group.conditions, metricIndex)
}

function buildMixedLogicFilterNode(
  groupId: string,
  conditions: StrategyCondition[],
  metricIndex: MetricIndex
): BuiltFilterNode {
  if (conditions.length === 0) {
    throw new StrategyRuleSpecError("指标组不能为空", { groupId })
  }

  const segments: BuiltFilterNode[][] = [[]]

  conditions.forEach((condition, index) => {
    const leaf: BuiltFilterNode = {
      conditionId: condition.id,
      expr: buildConditionFilterExpr(condition, metricIndex),
      groupId,
      type: "leaf",
    }

    if (index === 0 || condition.logic === "and") {
      segments[segments.length - 1].push(leaf)
      return
    }

    segments.push([leaf])
  })

  const segmentNodes = segments.map((segment) =>
    segment.length === 1
      ? segment[0]
      : {
          children: segment,
          type: "all" as const,
        }
  )

  return segmentNodes.length === 1
    ? segmentNodes[0]
    : {
        children: segmentNodes,
        type: "any",
      }
}

function buildConditionFilterExpr(
  condition: StrategyCondition,
  metricIndex: MetricIndex
): FilterExpr {
  const leftMetric = requireMetric(condition.metric, metricIndex, {
    conditionId: condition.id,
  })
  if (!leftMetric.allow_filter) {
    throw new StrategyRuleSpecError(`指标不可用于筛选: ${condition.metric}`, {
      conditionId: condition.id,
    })
  }

  const op = condition.operator
  if (!leftMetric.allowed_ops.includes(op)) {
    throw new StrategyRuleSpecError(
      `指标不支持操作符: ${condition.metric} ${op}`,
      {
        conditionId: condition.id,
      }
    )
  }

  const left: Operand = { type: "metric", name: leftMetric.logical_metric }

  if (op === "is_null") {
    return { type: "compare", left, op }
  }

  const right = buildRightOperand(condition, leftMetric, op, metricIndex)
  return { type: "compare", left, op, right }
}

function buildRightOperand(
  condition: StrategyCondition,
  leftMetric: MetricDefinition,
  op: Operator,
  metricIndex: MetricIndex
): Operand {
  if (op === "between") {
    return {
      type: "range",
      min: {
        type: "number",
        value: parseNumber(condition.value, condition.id),
      },
      max: {
        type: "number",
        value: parseNumber(condition.valueEnd, condition.id),
      },
    }
  }

  if (condition.target === "metric") {
    const rightMetric = requireMetric(condition.compareMetric, metricIndex, {
      conditionId: condition.id,
    })
    if (!rightMetric.allow_filter) {
      throw new StrategyRuleSpecError(
        `对比指标不可用于筛选: ${rightMetric.logical_metric}`,
        {
          conditionId: condition.id,
        }
      )
    }
    validateMetricOperandPair(condition, leftMetric, rightMetric, op)
    return { type: "metric", name: rightMetric.logical_metric }
  }

  if (op === "crosses_above" || op === "crosses_below") {
    validateCrossingMetric(leftMetric, condition.id)
    return { type: "number", value: parseNumber(condition.value, condition.id) }
  }

  return buildLiteralOperand(condition, leftMetric)
}

function buildLiteralOperand(
  condition: StrategyCondition,
  leftMetric: MetricDefinition
): Operand {
  switch (leftMetric.value_kind) {
    case "numeric":
    case "integer":
      return {
        type: "number",
        value: parseNumber(condition.value, condition.id),
      }
    case "boolean":
      return {
        type: "bool",
        value: parseBool(condition.value, condition.id),
      }
    case "string":
      return {
        type: "string",
        value: condition.value,
      }
    case "date":
      throw new StrategyRuleSpecError("日期指标暂只支持为空判断", {
        conditionId: condition.id,
      })
  }
}

function validateMetricOperandPair(
  condition: StrategyCondition,
  leftMetric: MetricDefinition,
  rightMetric: MetricDefinition,
  op: Operator
) {
  if (op === "crosses_above" || op === "crosses_below") {
    validateCrossingMetric(leftMetric, condition.id)
    validateCrossingMetric(rightMetric, condition.id)
    return
  }

  const leftKind = leftMetric.value_kind
  const rightKind = rightMetric.value_kind
  const compatible =
    leftKind === rightKind ||
    (isNumericKind(leftKind) && isNumericKind(rightKind))

  if (!compatible) {
    throw new StrategyRuleSpecError(
      `指标类型不兼容: ${leftMetric.logical_metric} / ${rightMetric.logical_metric}`,
      { conditionId: condition.id }
    )
  }
}

function validateCrossingMetric(metric: MetricDefinition, conditionId: string) {
  if (!isNumericKind(metric.value_kind) || !metric.cross?.previous_metric) {
    throw new StrategyRuleSpecError(
      `指标不支持上穿/下穿: ${metric.logical_metric}`,
      {
        conditionId,
      }
    )
  }
}

function requireMetric(
  metricName: string,
  metricIndex: MetricIndex,
  options: { conditionId?: string } = {}
) {
  const metric = metricIndex.get(metricName)
  if (!metric) {
    throw new StrategyRuleSpecError(`指标不存在: ${metricName}`, options)
  }

  return metric
}

function parseNumber(value: string, conditionId: string) {
  const numberValue = Number(value)
  if (!Number.isFinite(numberValue)) {
    throw new StrategyRuleSpecError(`数值无效: ${value}`, { conditionId })
  }

  return numberValue
}

function parseBool(value: string, conditionId: string) {
  if (value === "true") {
    return true
  }
  if (value === "false") {
    return false
  }

  throw new StrategyRuleSpecError(`布尔值无效: ${value}`, { conditionId })
}

function isNumericKind(valueKind: MetricValueKind) {
  return valueKind === "numeric" || valueKind === "integer"
}

function toFilterExpr(
  node: BuiltFilterNode,
  path: string,
  conditionPaths: ConditionFilterPath[]
): FilterExpr {
  if (node.type === "leaf") {
    conditionPaths.push({
      conditionId: node.conditionId,
      groupId: node.groupId,
      path,
    })
    return node.expr
  }

  return {
    type: node.type,
    conditions: node.children.map((child, index) =>
      toFilterExpr(child, `${path}.conditions.${index}`, conditionPaths)
    ),
  }
}

function normalizeTopN(topN: number | undefined) {
  if (!topN || !Number.isFinite(topN)) {
    return 20
  }

  return Math.max(1, Math.floor(topN))
}

function buildOutputMetrics(
  conditionGroups: StrategyConditionGroup[],
  metricIndex: MetricIndex
) {
  const output = new Set<string>()

  for (const metric of metricIndex.values()) {
    if (metric.default_output) {
      output.add(metric.logical_metric)
    }
  }

  for (const group of conditionGroups) {
    for (const condition of group.conditions) {
      const leftMetric = metricIndex.get(condition.metric)
      if (leftMetric) {
        output.add(leftMetric.logical_metric)
        if (isCrossingOperator(condition.operator)) {
          addPreviousMetric(output, leftMetric)
        }
      }

      if (condition.target === "metric") {
        const rightMetric = metricIndex.get(condition.compareMetric)
        if (rightMetric) {
          output.add(rightMetric.logical_metric)
          if (isCrossingOperator(condition.operator)) {
            addPreviousMetric(output, rightMetric)
          }
        }
      }
    }
  }

  return [...output].sort()
}

function addPreviousMetric(output: Set<string>, metric: MetricDefinition) {
  if (metric.cross?.previous_metric) {
    output.add(metric.cross.previous_metric)
  }
}

function isCrossingOperator(operator: ConditionOperator) {
  return operator === "crosses_above" || operator === "crosses_below"
}
