import type {
  IndicatorCatalog,
  SimulationSettings,
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import {
  formatComparableIndicator,
  formatWeightIndicator,
  getOperatorLabel,
} from "@/features/strategy/utils"
import type {
  BacktestExecutionConfig,
  FilterExpr,
  JsonValue,
  Operand,
  Operator,
  RuleVersionSpec,
  ScoringRule,
} from "@/types/rearview"

export const STRATEGY_CONFIG_DISPLAY_VERSION = 1

export type StrategyConfigConditionRow = {
  expression: string
  groupLabel: string
  id: string
  logicLabel: string
}

export type StrategyConfigScoringRow = {
  expression: string
  id: string
  index: number
  score: number
}

export type StrategyConfigSummaryRow = {
  label: string
  value: string
}

export type StrategyConfigDisplayModel = {
  buildSummaryRows: StrategyConfigSummaryRow[]
  conditionRows: StrategyConfigConditionRow[]
  scoringRows: StrategyConfigScoringRow[]
  version: typeof STRATEGY_CONFIG_DISPLAY_VERSION
}

export type StrategyConfigSourceContext = {
  benchmarkLabel: string
  benchmarkSecurityCode: string
  liveStartDate: string
  periodKey: string
  periodLabel: string
  sourceEndDate: string
  sourceStartDate: string
}

export function buildStrategyConfigDisplayFromDraft({
  conditionCatalogOptions,
  conditionGroups,
  scoringCatalogOptions,
  settings,
  weightIndicators,
}: {
  conditionCatalogOptions: IndicatorCatalog[]
  conditionGroups: StrategyConditionGroup[]
  scoringCatalogOptions: IndicatorCatalog[]
  settings: SimulationSettings
  weightIndicators: WeightIndicator[]
}): StrategyConfigDisplayModel {
  return {
    version: STRATEGY_CONFIG_DISPLAY_VERSION,
    conditionRows: conditionGroups.flatMap((group, groupIndex) =>
      group.conditions.map((condition, conditionIndex) => ({
        id: condition.id,
        expression: formatComparableIndicator(condition, {
          catalogOptions: conditionCatalogOptions,
        }),
        groupLabel: group.name || `指标组 ${groupIndex + 1}`,
        logicLabel:
          conditionIndex === 0 ? "组内起始" : condition.logic.toUpperCase(),
      }))
    ),
    scoringRows: weightIndicators.map((indicator, index) => ({
      id: indicator.id,
      expression: formatWeightIndicator(indicator, {
        catalogOptions: scoringCatalogOptions,
      }),
      index: index + 1,
      score: indicator.score,
    })),
    buildSummaryRows: buildStrategyConfigSummaryRows(settings),
  }
}

export function buildStrategyConfigDisplayFromCanonical({
  executionConfig,
  metricCatalogs = [],
  rule,
}: {
  executionConfig: BacktestExecutionConfig
  metricCatalogs?: IndicatorCatalog[]
  rule: RuleVersionSpec
}): StrategyConfigDisplayModel {
  const metricLabels = buildMetricLabelMap(metricCatalogs)

  return {
    version: STRATEGY_CONFIG_DISPLAY_VERSION,
    conditionRows: buildConditionRowsFromCanonical(
      rule.pool_filters,
      metricLabels
    ),
    scoringRows: buildScoringRowsFromCanonical(rule.scoring.rules, metricLabels),
    buildSummaryRows: buildExecutionConfigSummaryRows(
      executionConfig,
      metricLabels
    ),
  }
}

export function readRuleVersionSpec(value: JsonValue): RuleVersionSpec | null {
  const record = asJsonRecord(value)
  if (!record) {
    return null
  }

  const universe = readUniverseSpec(record.universe)
  const poolFilters = readFilterExpr(record.pool_filters)
  const scoring = readScoringSpec(record.scoring)
  const topNDefault = readFiniteNumber(record.top_n_default)
  const outputMetrics = readStringArray(record.output_metrics)

  if (
    !universe ||
    !poolFilters ||
    !scoring ||
    topNDefault === null ||
    !outputMetrics
  ) {
    return null
  }

  return {
    universe,
    pool_filters: poolFilters,
    scoring,
    top_n_default: topNDefault,
    output_metrics: outputMetrics,
  }
}

export function buildStrategyConfigSourceContext({
  benchmark_security_code,
  live_start_date,
  source_end_date,
  source_period_key,
  source_start_date,
}: {
  benchmark_security_code: string
  live_start_date: string
  source_end_date: string
  source_period_key: string
  source_start_date: string
}): StrategyConfigSourceContext {
  return {
    benchmarkLabel: formatBenchmarkLabel(benchmark_security_code),
    benchmarkSecurityCode: benchmark_security_code,
    liveStartDate: live_start_date,
    periodKey: source_period_key,
    periodLabel: formatBacktestPeriodLabel(source_period_key),
    sourceEndDate: source_end_date,
    sourceStartDate: source_start_date,
  }
}

function buildStrategyConfigSummaryRows(
  settings: SimulationSettings
): StrategyConfigSummaryRow[] {
  const riskRules = buildDraftRiskRuleLabels(settings)

  return [
    { label: "初始资金", value: formatCurrency(settings.initialCapital) },
    { label: "每日候选", value: `Top ${settings.buyTopN}` },
    {
      label: "候选口径",
      value: "Top N 是每日候选信号，不是目标持仓集合",
    },
    { label: "调仓规则", value: "仅空位调入；旧持仓由风控退出" },
    { label: "最大持仓", value: `${settings.maxPositions} 只` },
    {
      label: "单票上限",
      value: formatUiPercent(settings.singlePositionLimitPercent),
    },
    {
      label: "交易成本",
      value: `佣金 ${formatUiPercent(
        settings.transactionFees.commissionRatePercent
      )} / 滑点 ${formatUiPercent(
        settings.transactionFees.slippageRatePercent
      )}`,
    },
    {
      label: "风控",
      value: riskRules.length > 0 ? riskRules.join("，") : "未启用",
    },
  ]
}

function buildDraftRiskRuleLabels(settings: SimulationSettings) {
  const rules: string[] = []
  if (settings.fixedStopLoss.enabled) {
    rules.push(`固定止损 ${formatUiPercent(settings.fixedStopLoss.lossPercent)}`)
  }
  if (settings.takeProfit.enabled) {
    rules.push(`止盈 ${formatUiPercent(settings.takeProfit.profitPercent)}`)
  }
  if (settings.timeStopLoss.enabled) {
    rules.push(`时间止损 ${settings.timeStopLoss.holdingDays} 天`)
  }
  if (settings.indicatorStopLoss.enabled) {
    rules.push(`指标止损 ${settings.indicatorStopLoss.metric}`)
  }
  return rules
}

function buildConditionRowsFromCanonical(
  filter: FilterExpr,
  metricLabels: Map<string, string>
): StrategyConfigConditionRow[] {
  const groups = splitCanonicalConditionGroups(filter)

  return groups.flatMap((group, groupIndex) =>
    buildRowsForFilterGroup(group, `条件组 ${groupIndex + 1}`, metricLabels)
  )
}

function splitCanonicalConditionGroups(filter: FilterExpr): FilterExpr[] {
  if (filter.type !== "all") {
    return [filter]
  }

  if (filter.conditions.length === 0) {
    return []
  }

  if (filter.conditions.every((condition) => condition.type === "compare")) {
    return [filter]
  }

  return filter.conditions
}

function buildRowsForFilterGroup(
  filter: FilterExpr,
  groupLabel: string,
  metricLabels: Map<string, string>
): StrategyConfigConditionRow[] {
  if (filter.type === "all" || filter.type === "any") {
    return filter.conditions.map((condition, index) => ({
      id: `${groupLabel}-${index + 1}`,
      expression: formatFilterExpr(condition, metricLabels),
      groupLabel,
      logicLabel:
        index === 0
          ? "组内起始"
          : filter.type === "all"
            ? "AND"
            : "OR",
    }))
  }

  return [
    {
      id: `${groupLabel}-1`,
      expression: formatFilterExpr(filter, metricLabels),
      groupLabel,
      logicLabel: "组内起始",
    },
  ]
}

function buildScoringRowsFromCanonical(
  rules: ScoringRule[],
  metricLabels: Map<string, string>
): StrategyConfigScoringRow[] {
  return rules.map((rule, index) => {
    if (rule.type === "conditional_points") {
      return {
        id: rule.name || `score-${index + 1}`,
        expression: formatFilterExpr(rule.condition, metricLabels),
        index: index + 1,
        score: rule.points,
      }
    }

    return {
      id: rule.name || `score-${index + 1}`,
      expression: `加权指标 ${formatMetricName(rule.metric, metricLabels)}`,
      index: index + 1,
      score: rule.weight,
    }
  })
}

function buildExecutionConfigSummaryRows(
  executionConfig: BacktestExecutionConfig,
  metricLabels: Map<string, string>
): StrategyConfigSummaryRow[] {
  const riskRules = executionConfig.risk_exit_policy.exit_rules.map((rule) => {
    if (rule.type === "fixed_stop_loss") {
      return `固定止损 ${formatDecimalPercent(rule.loss_pct)}`
    }
    if (rule.type === "take_profit") {
      return `止盈 ${formatDecimalPercent(rule.profit_pct)}`
    }
    if (rule.type === "time_stop_loss") {
      return `时间止损 ${rule.holding_days} 天`
    }
    return `指标止损 ${formatMetricName(rule.metric, metricLabels)}`
  })

  return [
    {
      label: "初始资金",
      value: formatCurrency(executionConfig.account.initial_cash),
    },
    {
      label: "每日候选",
      value: `Top ${executionConfig.signal_policy.buy_signal_top_n}`,
    },
    {
      label: "候选口径",
      value: "Top N 是每日候选信号，不是目标持仓集合",
    },
    { label: "调仓规则", value: "仅空位调入；旧持仓由风控退出" },
    {
      label: "最大持仓",
      value: `${executionConfig.rebalance_policy.max_positions} 只`,
    },
    {
      label: "单票上限",
      value: formatDecimalPercent(
        executionConfig.rebalance_policy.single_position_limit_pct
      ),
    },
    {
      label: "交易成本",
      value: `佣金 ${formatDecimalPercent(
        executionConfig.fee_profile.commission_rate
      )} / 滑点 ${formatBpsPercent(
        Math.max(
          executionConfig.slippage_profile.buy_bps,
          executionConfig.slippage_profile.sell_bps
        )
      )}`,
    },
    {
      label: "风控",
      value: riskRules.length > 0 ? riskRules.join("，") : "未启用",
    },
  ]
}

function formatFilterExpr(
  filter: FilterExpr,
  metricLabels: Map<string, string>
): string {
  if (filter.type === "compare") {
    return formatCompareExpr(filter, metricLabels)
  }

  if (filter.type === "not") {
    return `非 (${formatFilterExpr(filter.condition, metricLabels)})`
  }

  const joiner = filter.type === "all" ? " 且 " : " 或 "
  return filter.conditions
    .map((condition) => formatNestedFilterExpr(condition, metricLabels))
    .join(joiner)
}

function formatNestedFilterExpr(
  filter: FilterExpr,
  metricLabels: Map<string, string>
): string {
  if (filter.type === "compare") {
    return formatCompareExpr(filter, metricLabels)
  }

  return `(${formatFilterExpr(filter, metricLabels)})`
}

function formatCompareExpr(
  filter: Extract<FilterExpr, { type: "compare" }>,
  metricLabels: Map<string, string>
) {
  const left = formatOperand(filter.left, metricLabels)
  const operatorLabel = getOperatorLabel(filter.op)

  if (filter.op === "is_null") {
    return `${left} ${operatorLabel}`
  }

  if (!filter.right) {
    return `${left} ${operatorLabel}`
  }

  return `${left} ${operatorLabel} ${formatOperand(filter.right, metricLabels)}`
}

function formatOperand(
  operand: Operand,
  metricLabels: Map<string, string>
): string {
  if (operand.type === "metric") {
    return formatMetricName(operand.name, metricLabels)
  }
  if (operand.type === "number") {
    return formatNumber(operand.value)
  }
  if (operand.type === "bool") {
    return operand.value ? "true" : "false"
  }
  if (operand.type === "string") {
    return operand.value
  }
  if (operand.type === "range") {
    return `${formatOperand(operand.min, metricLabels)} - ${formatOperand(
      operand.max,
      metricLabels
    )}`
  }

  return `${formatOperand(operand.left, metricLabels)} * ${formatOperand(
    operand.right,
    metricLabels
  )}`
}

function buildMetricLabelMap(catalogs: IndicatorCatalog[]) {
  const labels = new Map<string, string>()

  for (const catalog of catalogs) {
    for (const metric of catalog.metrics) {
      if (!labels.has(metric.id)) {
        labels.set(metric.id, metric.label)
      }
    }
  }

  return labels
}

function formatMetricName(metric: string, metricLabels: Map<string, string>) {
  return metricLabels.get(metric) ?? metric
}

function formatBacktestPeriodLabel(periodKey: string) {
  const labels: Record<string, string> = {
    "1y": "近一年",
    "2y": "近两年",
    "3y": "近三年",
  }

  return labels[periodKey] ?? periodKey
}

function formatBenchmarkLabel(securityCode: string) {
  const labels: Record<string, string> = {
    "000903.SH": "中证A100",
    "000300.SH": "沪深300",
    "000905.SH": "中证500",
    "000906.SH": "中证800",
    "000852.SH": "中证1000",
    "399311.SZ": "国证1000",
  }

  return labels[securityCode] ?? securityCode
}

function readUniverseSpec(value: JsonValue | undefined) {
  const record = asJsonRecord(value)
  const base = readString(record?.base)
  const excludeSt = readBoolean(record?.exclude_st)
  const excludeSuspend = readBoolean(record?.exclude_suspend)
  const includeSecurityCodes = readStringArray(record?.include_security_codes)
  const excludeSecurityCodes = readStringArray(record?.exclude_security_codes)

  if (
    !base ||
    excludeSt === null ||
    excludeSuspend === null ||
    !includeSecurityCodes ||
    !excludeSecurityCodes
  ) {
    return null
  }

  return {
    base,
    exclude_st: excludeSt,
    exclude_suspend: excludeSuspend,
    include_security_codes: includeSecurityCodes,
    exclude_security_codes: excludeSecurityCodes,
  }
}

function readScoringSpec(value: JsonValue | undefined) {
  const record = asJsonRecord(value)
  const rules = readScoringRules(record?.rules)
  const clamp = asJsonRecord(record?.clamp)
  const min = readFiniteNumber(clamp?.min)
  const max = readFiniteNumber(clamp?.max)

  if (!rules || min === null || max === null) {
    return null
  }

  return {
    rules,
    clamp: { min, max },
  }
}

function readScoringRules(value: JsonValue | undefined) {
  if (!Array.isArray(value)) {
    return null
  }

  const rules: ScoringRule[] = []
  for (const item of value) {
    const record = asJsonRecord(item)
    const type = readString(record?.type)
    const name = readString(record?.name)

    if (!name) {
      return null
    }

    if (type === "conditional_points") {
      const condition = readFilterExpr(record?.condition)
      const points = readFiniteNumber(record?.points)
      if (!condition || points === null) {
        return null
      }
      rules.push({ type, name, condition, points })
      continue
    }

    if (type === "weighted_metric") {
      const metric = readString(record?.metric)
      const weight = readFiniteNumber(record?.weight)
      if (!metric || weight === null) {
        return null
      }
      rules.push({ type, name, metric, weight })
      continue
    }

    return null
  }

  return rules
}

function readFilterExpr(value: JsonValue | undefined): FilterExpr | null {
  const record = asJsonRecord(value)
  const type = readString(record?.type)

  if (type === "all" || type === "any") {
    const conditions = readFilterExprArray(record?.conditions)
    return conditions ? { type, conditions } : null
  }

  if (type === "not") {
    const condition = readFilterExpr(record?.condition)
    return condition ? { type, condition } : null
  }

  if (type === "compare") {
    const left = readOperand(record?.left)
    const op = readString(record?.op)
    if (!left || !isOperator(op)) {
      return null
    }
    if (record?.right === undefined || record.right === null) {
      return { type, left, op }
    }
    const right = readOperand(record.right)
    return right ? { type, left, op, right } : null
  }

  return null
}

function readFilterExprArray(value: JsonValue | undefined) {
  if (!Array.isArray(value)) {
    return null
  }

  const conditions: FilterExpr[] = []
  for (const item of value) {
    const condition = readFilterExpr(item)
    if (!condition) {
      return null
    }
    conditions.push(condition)
  }
  return conditions
}

function readOperand(value: JsonValue | undefined): Operand | null {
  const record = asJsonRecord(value)
  const type = readString(record?.type)

  if (type === "metric") {
    const name = readString(record?.name)
    return name ? { type, name } : null
  }
  if (type === "number") {
    const numberValue = readFiniteNumber(record?.value)
    return numberValue === null ? null : { type, value: numberValue }
  }
  if (type === "bool") {
    const boolValue = readBoolean(record?.value)
    return boolValue === null ? null : { type, value: boolValue }
  }
  if (type === "string") {
    const stringValue = readString(record?.value)
    return stringValue === null ? null : { type, value: stringValue }
  }
  if (type === "range") {
    const min = readOperand(record?.min)
    const max = readOperand(record?.max)
    return min && max ? { type, min, max } : null
  }
  if (type === "binary") {
    const op = readString(record?.op)
    const left = readOperand(record?.left)
    const right = readOperand(record?.right)
    return op === "multiply" && left && right ? { type, op, left, right } : null
  }

  return null
}

function asJsonRecord(value: JsonValue | undefined) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null
  }

  return value
}

function readString(value: JsonValue | undefined) {
  return typeof value === "string" ? value : null
}

function readStringArray(value: JsonValue | undefined) {
  if (!Array.isArray(value)) {
    return null
  }

  const strings: string[] = []
  for (const item of value) {
    if (typeof item !== "string") {
      return null
    }
    strings.push(item)
  }
  return strings
}

function readBoolean(value: JsonValue | undefined) {
  return typeof value === "boolean" ? value : null
}

function readFiniteNumber(value: JsonValue | undefined) {
  return typeof value === "number" && Number.isFinite(value) ? value : null
}

function isOperator(value: string | null): value is Operator {
  return (
    value === "eq" ||
    value === "ne" ||
    value === "lt" ||
    value === "lte" ||
    value === "gt" ||
    value === "gte" ||
    value === "between" ||
    value === "is_null" ||
    value === "crosses_above" ||
    value === "crosses_below"
  )
}

function formatCurrency(value: number) {
  return `¥${value.toFixed(2)}`
}

function formatDecimalPercent(value: number) {
  return formatUiPercent(value * 100)
}

function formatBpsPercent(value: number) {
  return formatUiPercent(value / 100)
}

function formatUiPercent(value: number) {
  return `${Number(value.toFixed(3))}%`
}

function formatNumber(value: number) {
  return Number(value.toFixed(6)).toString()
}
