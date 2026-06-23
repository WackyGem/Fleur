import { useMemo, useState } from "react"
import { useNavigate } from "react-router-dom"
import { useQueryClient } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import { securityAnalysis } from "@/api/rearview"
import {
  useDefaultMarketFeeTemplateQuery,
  useMetricsQuery,
  useStrategyBacktestValidateQuery,
  useStrategyPreviewMutation,
  useStrategyPreviewTimelineMutation,
} from "@/api/hooks"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Skeleton } from "@/components/ui/skeleton"
import { Spinner } from "@/components/ui/spinner"
import {
  buildStrategyMetricCatalog,
  buildStrategyPreviewRuleSpec,
  buildStrategyScoringCatalog,
  StrategyRuleSpecError,
} from "@/features/strategy/adapters"
import { ConditionGroupsPanel } from "@/features/strategy/components/condition-groups-panel"
import { PoolPreviewPanel } from "@/features/strategy/components/pool-preview-panel"
import { SimulationPositionPanel } from "@/features/strategy/components/simulation-position-panel"
import { strategySplitPanelColumnsClassName } from "@/features/strategy/components/strategy-split-layout"
import { StrategySplitPanel } from "@/features/strategy/components/strategy-split-panel"
import { StrategyStepSidebar } from "@/features/strategy/components/strategy-step-sidebar"
import { WeightIndicatorsPanel } from "@/features/strategy/components/weight-indicators-panel"
import {
  areTransactionFeesEqual,
  buildBacktestExecutionRequestDraft,
  buildStrategyBacktestValidateRequest,
  marketTemplateToTransactionFees,
  toBacktestExecutionDraft,
  type BacktestExecutionDraft,
  type BacktestPeriodValue,
} from "@/features/strategy/execution"
import {
  buildPreviewSnapshot,
  buildPreviewTimelineRange,
  markPreviewSnapshotStale,
  type PreviewRange,
  type PreviewSnapshot,
} from "@/features/strategy/preview"
import type {
  SimulationSettings,
  Step,
  StrategyCondition,
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import {
  createCondition,
  createId,
  createWeightIndicator,
} from "@/features/strategy/utils"
import { cn } from "@/lib/utils"
import { ArrowRight } from "lucide-react"

const stepContent: Record<Step, { description: string; title: string }> = {
  indicators: {
    title: "策略选股",
    description: "组间固定 AND，组内每个指标前可配置 AND / OR",
  },
  weights: {
    title: "权重配置",
    description: "按指标配置权重，权重将在策略评分时使用",
  },
  preview: {
    title: "股池预览",
    description: "预览当前指标条件与权重配置生成的候选股池口径",
  },
  simulation: {
    title: "模拟建仓",
    description: "配置建仓资金、买入触发和卖出条件",
  },
  backtest: {
    title: "策略回测",
    description: "调整回测配置，查看净值走势、持仓记录与策略业绩",
  },
}

const defaultPreviewWeightIndicators: WeightIndicator[] = [
  {
    id: "preview-weight-trend",
    catalogId: "trend",
    metric: "price_ma_20",
    target: "value",
    operator: "gte",
    value: "0",
    valueEnd: "",
    compareCatalogId: "quotes",
    compareMetric: "close_price",
    score: 42,
  },
  {
    id: "preview-weight-momentum",
    catalogId: "momentum",
    metric: "kdj_j_value",
    target: "value",
    operator: "gte",
    value: "50",
    valueEnd: "",
    compareCatalogId: "quotes",
    compareMetric: "close_price",
    score: 34,
  },
  {
    id: "preview-weight-volume",
    catalogId: "volume",
    metric: "volume_ma_5",
    target: "value",
    operator: "gte",
    value: "0",
    valueEnd: "",
    compareCatalogId: "quotes",
    compareMetric: "volume",
    score: 24,
  },
]

const defaultSimulationSettings: SimulationSettings = {
  initialCapital: 1_000_000,
  buyTopN: 10,
  singlePositionLimitPercent: 10,
  transactionFees: {
    stampDutyRatePercent: 0.05,
    transferFeeRatePercent: 0.001,
    commissionRatePercent: 0.01,
    commissionRateMaxPercent: 0.3,
    minCommission: 5,
    buySlippageRatePercent: 0.1,
    sellSlippageRatePercent: 0.1,
  },
  fixedStopLoss: {
    enabled: false,
    lossPercent: 8,
  },
  indicatorStopLoss: {
    enabled: false,
    catalogId: "trend",
    metric: "price_ma_10",
  },
  takeProfit: {
    enabled: false,
    profitPercent: 20,
  },
  timeStopLoss: {
    enabled: false,
    holdingDays: 20,
    minimumReturnPercent: 0,
  },
}

const backtestPeriodOptions = [
  { value: "1y", label: "近一年" },
  { value: "2y", label: "近两年" },
  { value: "3y", label: "近三年" },
] as const

const backtestBenchmarkOptions = [
  { securityCode: "000903.SH", label: "中证A100" },
  { securityCode: "000300.SH", label: "沪深300" },
  { securityCode: "000905.SH", label: "中证500" },
  { securityCode: "000906.SH", label: "中证800" },
  { securityCode: "000852.SH", label: "中证1000" },
  { securityCode: "399311.SZ", label: "国证1000" },
] as const

const splitStepLayoutClassName = strategySplitPanelColumnsClassName
const previewAnalysisMaWindows = "5,10,30"

type BacktestPeriod = BacktestPeriodValue
type BacktestBenchmark =
  (typeof backtestBenchmarkOptions)[number]["securityCode"]

function buildPreviewWeightIndicators(weightIndicators: WeightIndicator[]) {
  const source =
    weightIndicators.length > 0
      ? weightIndicators
      : defaultPreviewWeightIndicators

  return source.map((indicator) => ({ ...indicator }))
}

function cloneWeightIndicators(weightIndicators: WeightIndicator[]) {
  return weightIndicators.map((indicator) => ({ ...indicator }))
}

function buildPreviewRequestRange(previewRange: PreviewRange) {
  if (!previewRange.startDate || !previewRange.endDate) {
    throw new StrategyRuleSpecError("股池预览需要开始日期和结束日期")
  }
  if (previewRange.startDate > previewRange.endDate) {
    throw new StrategyRuleSpecError("股池预览开始日期不能晚于结束日期")
  }

  return {
    start_date: previewRange.startDate,
    end_date: previewRange.endDate,
    preview_row_limit: Math.max(
      1,
      Math.floor(previewRange.previewRowLimit || 1)
    ),
  }
}

function BacktestPanel({
  benchmark,
  draft,
  onBenchmarkChange,
  onPeriodChange,
  period,
}: {
  benchmark: BacktestBenchmark
  draft: BacktestExecutionDraft | null
  onBenchmarkChange: (benchmark: BacktestBenchmark) => void
  onPeriodChange: (period: BacktestPeriod) => void
  period: BacktestPeriod
}) {
  const selectedPeriodLabel =
    backtestPeriodOptions.find((option) => option.value === period)?.label ??
    period
  const selectedBenchmarkLabel =
    backtestBenchmarkOptions.find((option) => option.securityCode === benchmark)
      ?.label ?? benchmark
  const requestDraft = draft
    ? buildBacktestExecutionRequestDraft({
        benchmark,
        draft,
        period,
      })
    : null

  return (
    <StrategySplitPanel
      main={
        <div className="flex w-full flex-col gap-4">
          <div className="text-sm font-medium">回测配置</div>
          <FieldGroup className="grid gap-3 md:grid-cols-3 md:items-end xl:pr-4">
            <Field>
              <FieldLabel>周期</FieldLabel>
              <Select
                value={period}
                onValueChange={(value) =>
                  onPeriodChange(value as BacktestPeriod)
                }
              >
                <SelectTrigger className="w-full bg-background">
                  <SelectValue>
                    <span className="truncate">{selectedPeriodLabel}</span>
                  </SelectValue>
                </SelectTrigger>
                <SelectContent align="start">
                  <SelectGroup>
                    {backtestPeriodOptions.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </Field>

            <Field>
              <FieldLabel>业绩比较基准</FieldLabel>
              <Select
                value={benchmark}
                onValueChange={(value) =>
                  onBenchmarkChange(value as BacktestBenchmark)
                }
              >
                <SelectTrigger className="w-full bg-background">
                  <SelectValue>
                    <span className="truncate">{selectedBenchmarkLabel}</span>
                  </SelectValue>
                </SelectTrigger>
                <SelectContent align="start">
                  <SelectGroup>
                    {backtestBenchmarkOptions.map((option) => (
                      <SelectItem
                        key={option.securityCode}
                        value={option.securityCode}
                      >
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </Field>

            <Button
              className="w-full"
              disabled
              variant="outline"
              size="lg"
              type="button"
            >
              真实回测待接入
            </Button>
          </FieldGroup>

          <Separator className="bg-border/60" />

          {!draft || !requestDraft ? (
            <Alert>
              <AlertTitle>缺少回测执行草稿</AlertTitle>
              <AlertDescription>
                回到 Step 4，等待 Rearview 完成 execution config 校验后再进入回测配置。
              </AlertDescription>
            </Alert>
          ) : (
            <section className="flex flex-col gap-4 xl:pr-4">
              <div className="grid gap-3 md:grid-cols-3">
                <BacktestSummaryMetric
                  label="回测区间"
                  value={`${requestDraft.start_date} 至 ${requestDraft.end_date}`}
                />
                <BacktestSummaryMetric
                  label="业绩基准"
                  value={selectedBenchmarkLabel}
                />
                <BacktestSummaryMetric
                  label="买入信号"
                  value={`Top ${requestDraft.top_n}`}
                />
                <BacktestSummaryMetric
                  label="初始资金"
                  value={formatCurrency(
                    draft.execution_config.account.initial_cash
                  )}
                />
                <BacktestSummaryMetric
                  label="单票目标"
                  value={formatDecimalPercent(
                    draft.summary.target_weight_per_position_pct
                  )}
                />
                <BacktestSummaryMetric
                  label="现金保留"
                  value={formatDecimalPercent(
                    draft.summary.implicit_cash_reserve_pct
                  )}
                />
              </div>

              <Separator className="bg-border/60" />

              <div className="flex flex-col gap-3">
                <div className="text-sm font-medium">Canonical 输入</div>
                <div className="grid gap-2 md:grid-cols-2">
                  <BacktestSummaryMetric
                    label="Rule hash"
                    value={formatHash(draft.rule_hash)}
                  />
                  <BacktestSummaryMetric
                    label="Execution config hash"
                    value={formatHash(draft.execution_config_hash)}
                  />
                  <BacktestSummaryMetric
                    label="Price basis"
                    value={draft.execution_config.price_basis}
                  />
                  <BacktestSummaryMetric
                    label="Signal timing"
                    value={draft.execution_config.signal_policy.signal_timing}
                  />
                </div>
              </div>

              <Separator className="bg-border/60" />

              <div className="flex flex-col gap-3">
                <div className="text-sm font-medium">卖出规则</div>
                {draft.execution_config.risk_exit_policy.exit_rules.length ===
                0 ? (
                  <div className="text-xs text-muted-foreground">
                    未启用卖出规则。
                  </div>
                ) : (
                  <div className="grid gap-2 md:grid-cols-2">
                    {draft.execution_config.risk_exit_policy.exit_rules.map(
                      (rule, index) => (
                        <BacktestSummaryMetric
                          key={`${rule.type}-${index}`}
                          label={`规则 ${index + 1}`}
                          value={formatExitRule(rule)}
                        />
                      )
                    )}
                  </div>
                )}
              </div>
            </section>
          )}
        </div>
      }
      aside={
        <Card className="h-fit bg-transparent py-0 ring-0">
          <CardHeader>
            <CardTitle>执行快照</CardTitle>
            <CardDescription>Step 5 不执行回测，仅确认输入边界</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            {!draft ? (
              <Alert>
                <AlertTitle>没有可用草稿</AlertTitle>
                <AlertDescription>
                  当前页面不会展示静态净值、持仓或绩效样例。
                </AlertDescription>
              </Alert>
            ) : (
              <>
                <BacktestSummaryMetric
                  label="Preview"
                  value={draft.preview_id ?? "未绑定"}
                />
                <BacktestSummaryMetric
                  label="Preview 区间"
                  value={
                    draft.preview_range
                      ? `${draft.preview_range.start_date} 至 ${draft.preview_range.end_date}`
                      : "未绑定"
                  }
                />
                <BacktestSummaryMetric
                  label="生成时间"
                  value={draft.createdAt}
                />
                <BacktestSummaryMetric
                  label="最大持仓"
                  value={`${draft.summary.max_positions} 只`}
                />
                <BacktestSummaryMetric
                  label="费用模板"
                  value={`${formatDecimalPercent(draft.execution_config.fee_profile.commission_rate)} 佣金`}
                />
              </>
            )}
          </CardContent>
        </Card>
      }
    />
  )
}

function BacktestSummaryMetric({
  label,
  value,
  valueClassName,
}: {
  label: string
  value: string
  valueClassName?: string
}) {
  return (
    <div className="min-w-0">
      <div className="truncate text-xs text-muted-foreground">{label}</div>
      <div
        className={cn(
          "mt-1 truncate text-sm font-medium tabular-nums",
          valueClassName
        )}
      >
        {value}
      </div>
    </div>
  )
}

function formatCurrency(value: number) {
  return `¥${value.toFixed(2)}`
}

function formatDecimalPercent(value: number) {
  return `${Number((value * 100).toFixed(2))}%`
}

function formatHash(value: string) {
  return value.length <= 16 ? value : `${value.slice(0, 10)}...${value.slice(-6)}`
}

function formatExitRule(
  rule: BacktestExecutionDraft["execution_config"]["risk_exit_policy"]["exit_rules"][number]
) {
  if (rule.type === "fixed_stop_loss") {
    return `固定止损 ${formatDecimalPercent(rule.loss_pct)}`
  }
  if (rule.type === "take_profit") {
    return `固定止盈 ${formatDecimalPercent(rule.profit_pct)}`
  }

  return `时间止损 ${rule.holding_days} 天 / ${formatDecimalPercent(rule.max_return_pct)}`
}

function formatErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message
  }

  return String(error || "Unknown error")
}

function MetricsCatalogState({
  error,
  isError,
  isLoading,
}: {
  error: unknown
  isError: boolean
  isLoading: boolean
}) {
  if (isError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>指标加载失败</AlertTitle>
        <AlertDescription>{formatErrorMessage(error)}</AlertDescription>
      </Alert>
    )
  }

  if (isLoading) {
    return <Skeleton className="h-9 w-full" />
  }

  return null
}

export function StrategyPage() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const metricsQuery = useMetricsQuery()
  const defaultMarketTemplateQuery =
    useDefaultMarketFeeTemplateQuery("CN_A_SHARE")
  const previewMutation = useStrategyPreviewMutation()
  const previewTimelineMutation = useStrategyPreviewTimelineMutation()
  const strategyCatalog = useMemo(
    () => buildStrategyMetricCatalog(metricsQuery.data ?? []),
    [metricsQuery.data]
  )
  const strategyScoringCatalog = useMemo(
    () => buildStrategyScoringCatalog(metricsQuery.data ?? []),
    [metricsQuery.data]
  )
  const hasRealMetricsCatalog =
    metricsQuery.isSuccess && strategyCatalog.length > 0
  const hasRealScoringCatalog =
    metricsQuery.isSuccess && strategyScoringCatalog.length > 0
  const strategyCatalogOptions = hasRealMetricsCatalog ? strategyCatalog : []
  const [activeStep, setActiveStep] = useState<Step>("indicators")
  const [conditionGroups, setConditionGroups] = useState<
    StrategyConditionGroup[]
  >([])
  const [weightIndicators, setWeightIndicators] = useState<WeightIndicator[]>(
    []
  )
  const [previewAppliedWeightIndicators, setPreviewAppliedWeightIndicators] =
    useState<WeightIndicator[]>(() => buildPreviewWeightIndicators([]))
  const [simulationSettings, setSimulationSettings] =
    useState<SimulationSettings>(defaultSimulationSettings)
  const [hasEditedTransactionFees, setHasEditedTransactionFees] =
    useState(false)
  const [backtestPeriod, setBacktestPeriod] = useState<BacktestPeriod>("1y")
  const [backtestBenchmark, setBacktestBenchmark] =
    useState<BacktestBenchmark>("000300.SH")
  const [previewAdapterError, setPreviewAdapterError] = useState<string | null>(
    null
  )
  const [isOpeningPreview, setIsOpeningPreview] = useState(false)
  const [previewSnapshot, setPreviewSnapshot] =
    useState<PreviewSnapshot | null>(null)
  const effectiveSimulationSettings = useMemo(() => {
    if (!defaultMarketTemplateQuery.data || hasEditedTransactionFees) {
      return simulationSettings
    }

    const transactionFees = marketTemplateToTransactionFees(
      defaultMarketTemplateQuery.data
    )

    return areTransactionFeesEqual(
      simulationSettings.transactionFees,
      transactionFees
    )
      ? simulationSettings
      : {
          ...simulationSettings,
          transactionFees,
        }
  }, [
    defaultMarketTemplateQuery.data,
    hasEditedTransactionFees,
    simulationSettings,
  ])
  const backtestValidateRequestState = useMemo(() => {
    if (!previewSnapshot || previewSnapshot.stale) {
      return { error: null, request: null }
    }
    if (!defaultMarketTemplateQuery.data) {
      return { error: null, request: null }
    }

    try {
      return {
        error: null,
        request: buildStrategyBacktestValidateRequest({
          marketTemplate: defaultMarketTemplateQuery.data,
          previewSnapshot,
          settings: effectiveSimulationSettings,
        }),
      }
    } catch (error) {
      return {
        error: formatErrorMessage(error),
        request: null,
      }
    }
  }, [
    defaultMarketTemplateQuery.data,
    effectiveSimulationSettings,
    previewSnapshot,
  ])
  const backtestValidateQuery = useStrategyBacktestValidateQuery(
    backtestValidateRequestState.request
  )
  const backtestExecutionDraft = useMemo<BacktestExecutionDraft | null>(() => {
    if (
      !backtestValidateRequestState.request ||
      !backtestValidateQuery.data
    ) {
      return null
    }

    return toBacktestExecutionDraft({
      createdAt: new Date(backtestValidateQuery.dataUpdatedAt).toISOString(),
      request: backtestValidateRequestState.request,
      response: backtestValidateQuery.data,
    })
  }, [
    backtestValidateQuery.data,
    backtestValidateQuery.dataUpdatedAt,
    backtestValidateRequestState.request,
  ])
  const backtestValidationError =
    backtestValidateRequestState.error ??
    (backtestValidateQuery.isError
      ? formatErrorMessage(backtestValidateQuery.error)
      : null)
  const isBacktestValidationPending = Boolean(
    backtestValidateRequestState.request && backtestValidateQuery.isFetching
  )
  const canEditConditions = strategyCatalogOptions.length > 0
  const canEditWeights = hasRealScoringCatalog

  function markRuleDraftChanged() {
    setPreviewAdapterError(null)
    setPreviewSnapshot(markPreviewSnapshotStale)
  }

  function handleSimulationSettingsChange(nextSettings: SimulationSettings) {
    if (
      !areTransactionFeesEqual(
        effectiveSimulationSettings.transactionFees,
        nextSettings.transactionFees
      )
    ) {
      setHasEditedTransactionFees(true)
    }
    setSimulationSettings(nextSettings)
  }

  function handleBack() {
    navigate("/dashboard", { viewTransition: true })
  }

  function createGroup() {
    if (!canEditConditions) {
      return
    }

    markRuleDraftChanged()
    setConditionGroups((current) => [
      ...current,
      {
        id: createId("group"),
        name: `指标组 ${current.length + 1}`,
        conditions: [createCondition(strategyCatalogOptions)],
      },
    ])
  }

  function updateGroup(
    groupId: string,
    patch: Partial<Pick<StrategyConditionGroup, "name">>
  ) {
    markRuleDraftChanged()
    setConditionGroups((current) =>
      current.map((group) =>
        group.id === groupId ? { ...group, ...patch } : group
      )
    )
  }

  function removeGroup(groupId: string) {
    markRuleDraftChanged()
    setConditionGroups((current) =>
      current.filter((group) => group.id !== groupId)
    )
  }

  function addCondition(groupId: string) {
    if (!canEditConditions) {
      return
    }

    markRuleDraftChanged()
    setConditionGroups((current) =>
      current.map((group) =>
        group.id === groupId
          ? {
              ...group,
              conditions: [
                ...group.conditions,
                createCondition(strategyCatalogOptions),
              ],
            }
          : group
      )
    )
  }

  function updateCondition(
    groupId: string,
    conditionId: string,
    patch: Partial<StrategyCondition>
  ) {
    markRuleDraftChanged()
    setConditionGroups((current) =>
      current.map((group) =>
        group.id === groupId
          ? {
              ...group,
              conditions: group.conditions.map((condition) =>
                condition.id === conditionId
                  ? { ...condition, ...patch }
                  : condition
              ),
            }
          : group
      )
    )
  }

  function removeCondition(groupId: string, conditionId: string) {
    markRuleDraftChanged()
    setConditionGroups((current) =>
      current.map((group) =>
        group.id === groupId
          ? {
              ...group,
              conditions: group.conditions.filter(
                (condition) => condition.id !== conditionId
              ),
            }
          : group
      )
    )
  }

  function addWeightIndicator() {
    if (!canEditWeights) {
      return
    }

    markRuleDraftChanged()
    setWeightIndicators((current) => [
      ...current,
      createWeightIndicator(strategyScoringCatalog),
    ])
  }

  function updateWeightIndicator(
    indicatorId: string,
    patch: Partial<WeightIndicator>
  ) {
    markRuleDraftChanged()
    setWeightIndicators((current) =>
      current.map((indicator) =>
        indicator.id === indicatorId ? { ...indicator, ...patch } : indicator
      )
    )
  }

  function removeWeightIndicator(indicatorId: string) {
    markRuleDraftChanged()
    setWeightIndicators((current) =>
      current.filter((indicator) => indicator.id !== indicatorId)
    )
  }

  async function openPreview(
    nextWeightIndicators: WeightIndicator[] = weightIndicators
  ) {
    setIsOpeningPreview(true)
    previewMutation.reset()
    previewTimelineMutation.reset()
    setPreviewAdapterError(null)

    try {
      if (!metricsQuery.data || metricsQuery.data.length === 0) {
        throw new StrategyRuleSpecError(
          "Rearview 指标目录未加载，不能执行股池预览"
        )
      }
      if (!hasRealScoringCatalog) {
        throw new StrategyRuleSpecError(
          "Rearview scoring 指标目录未加载，不能执行股池预览"
        )
      }

      const requestRange = buildPreviewRequestRange(buildPreviewTimelineRange())
      const previewWeights = cloneWeightIndicators(nextWeightIndicators)
      const { conditionPaths, rule } = buildStrategyPreviewRuleSpec(
        conditionGroups,
        previewWeights,
        metricsQuery.data
      )
      const timeline = await previewTimelineMutation.mutateAsync({
        end_date: requestRange.end_date,
        rule,
        start_date: requestRange.start_date,
      })
      const latestTradeDate = timeline.trade_dates.at(-1)?.trade_date ?? null
      const result = latestTradeDate
        ? await previewMutation.mutateAsync({
            end_date: latestTradeDate,
            preview_row_limit: requestRange.preview_row_limit,
            rule,
            start_date: latestTradeDate,
          })
        : {
            end_date: requestRange.end_date,
            preview_id: timeline.preview_id,
            preview_row_limit: requestRange.preview_row_limit,
            required_columns: timeline.required_columns,
            required_marts: timeline.required_marts,
            required_metrics: timeline.required_metrics,
            sql_hash: timeline.sql_hash,
            start_date: requestRange.start_date,
            top_n: rule.top_n_default,
            trade_dates: [],
          }

      setPreviewAppliedWeightIndicators(previewWeights)
      const nextPreviewSnapshot = buildPreviewSnapshot({
        appliedRuleSpec: rule,
        conditionGroups,
        conditionPaths,
        createdAt: new Date().toISOString(),
        metrics: metricsQuery.data,
        range: {
          endDate: requestRange.end_date,
          previewRowLimit: requestRange.preview_row_limit,
          selectedTradeDate: latestTradeDate,
          startDate: requestRange.start_date,
        },
        result,
        timeline,
        weightIndicators: previewWeights,
      })
      await prefetchInitialSecurityAnalysis(nextPreviewSnapshot)
      setPreviewSnapshot(nextPreviewSnapshot)
      setActiveStep("preview")
    } catch (error) {
      setPreviewAdapterError(formatErrorMessage(error))
      if (error instanceof StrategyRuleSpecError) {
        return
      }
    } finally {
      setIsOpeningPreview(false)
    }
  }

  async function prefetchInitialSecurityAnalysis(snapshot: PreviewSnapshot) {
    const latestTradeDate = snapshot.result.trade_dates.at(-1)
    const firstSignal = latestTradeDate?.signals[0]

    if (!latestTradeDate || !firstSignal) {
      return
    }

    const request = {
      adjustment: "forward_adjusted" as const,
      include_quote_rows: false,
      lookback_trading_days: 240,
      ma_windows: previewAnalysisMaWindows,
      security_code: firstSignal.security_code,
      trade_date: latestTradeDate.trade_date,
    }

    const queryKey = queryKeys.previewSecurityAnalysis(
      snapshot.previewId,
      request.trade_date,
      request.security_code,
      request.adjustment,
      request.ma_windows,
      request.include_quote_rows
    )

    try {
      queryClient.setQueryData(queryKey, await securityAnalysis(request))
    } catch {
      // Step3 will issue the same request through useQuery and retry normally.
    }
  }

  function changeStep(step: Step) {
    if (step === "preview") {
      void openPreview()
      return
    }
    if (step === "simulation" && !canEnterSimulation) {
      return
    }
    if (step === "backtest" && !canEnterBacktest) {
      return
    }

    setActiveStep(step)
  }

  const content = stepContent[activeStep]
  const isPreviewPending =
    isOpeningPreview ||
    previewMutation.isPending ||
    previewTimelineMutation.isPending
  const isSplitStep =
    activeStep === "preview" ||
    activeStep === "simulation" ||
    activeStep === "backtest"
  const showStepActions = activeStep !== "backtest"
  const canEnterSimulation = Boolean(
    previewSnapshot &&
    !previewSnapshot.stale &&
    previewSnapshot.result.trade_dates.some(
      (tradeDate) => tradeDate.pool_count > 0 && tradeDate.signals.length > 0
    )
  )
  const canEnterBacktest = Boolean(
    canEnterSimulation &&
      backtestExecutionDraft &&
      !backtestValidationError &&
      !isBacktestValidationPending &&
      !defaultMarketTemplateQuery.isLoading &&
      !defaultMarketTemplateQuery.isError
  )

  return (
    <section className="min-h-[calc(100svh-8rem)]">
      <div className="grid min-h-[calc(100svh-8rem)] grid-cols-1 lg:grid-cols-[1fr_9fr]">
        <StrategyStepSidebar
          activeStep={activeStep}
          onBack={handleBack}
          onStepChange={changeStep}
        />

        <main className="pt-4 lg:pt-0 lg:pl-6">
          <div className="flex h-[calc(100svh-8rem)] min-h-0 flex-col">
            <header className="flex h-9 items-center justify-between gap-3">
              <div>
                <h2 className="text-lg font-medium">{content.title}</h2>
                {content.description ? (
                  <div className="mt-1 text-xs text-muted-foreground">
                    {content.description}
                  </div>
                ) : null}
              </div>
            </header>

            <Separator className="mt-5" />

            <div
              className={cn(
                "min-h-0 flex-1 pr-1",
                isSplitStep && "[scrollbar-gutter:stable]",
                !isSplitStep && "mt-5",
                activeStep === "preview"
                  ? "overflow-y-auto xl:overflow-hidden"
                  : activeStep !== "weights" && "overflow-y-auto"
              )}
            >
              {activeStep === "indicators" ? (
                <div className="flex flex-col gap-3">
                  <MetricsCatalogState
                    error={metricsQuery.error}
                    isError={metricsQuery.isError}
                    isLoading={metricsQuery.isLoading}
                  />

                  {canEditConditions ? (
                    <ConditionGroupsPanel
                      catalogOptions={strategyCatalogOptions}
                      conditionGroups={conditionGroups}
                      onAddCondition={addCondition}
                      onCreateGroup={createGroup}
                      onRemoveCondition={removeCondition}
                      onRemoveGroup={removeGroup}
                      onUpdateCondition={updateCondition}
                      onUpdateGroup={updateGroup}
                    />
                  ) : metricsQuery.isLoading || metricsQuery.isError ? null : (
                    <Alert>
                      <AlertTitle>没有可筛选指标</AlertTitle>
                      <AlertDescription>
                        Rearview catalog 当前没有返回 allow_filter 指标。
                      </AlertDescription>
                    </Alert>
                  )}
                </div>
              ) : activeStep === "weights" ? (
                canEditWeights ? (
                  <div className="flex h-full min-h-0 flex-col gap-3">
                    <WeightIndicatorsPanel
                      catalogOptions={strategyScoringCatalog}
                      weightIndicators={weightIndicators}
                      onAddIndicator={addWeightIndicator}
                      onRemoveIndicator={removeWeightIndicator}
                      onUpdateIndicator={updateWeightIndicator}
                    />
                    {previewAdapterError || previewMutation.isError ? (
                      <Alert variant="destructive" className="shrink-0">
                        <AlertTitle>股池预览失败</AlertTitle>
                        <AlertDescription>
                          {previewAdapterError ??
                            formatErrorMessage(previewMutation.error)}
                        </AlertDescription>
                      </Alert>
                    ) : null}
                  </div>
                ) : (
                  <Alert>
                    <AlertTitle>没有可评分指标</AlertTitle>
                    <AlertDescription>
                      Rearview catalog 当前没有返回 allow_scoring 指标。
                    </AlertDescription>
                  </Alert>
                )
              ) : activeStep === "preview" ? (
                <PoolPreviewPanel
                  appliedWeightIndicators={previewAppliedWeightIndicators}
                  conditionGroups={conditionGroups}
                  error={
                    previewAdapterError ??
                    (previewMutation.isError
                      ? formatErrorMessage(previewMutation.error)
                      : previewTimelineMutation.isError
                        ? formatErrorMessage(previewTimelineMutation.error)
                        : null)
                  }
                  isPending={isPreviewPending}
                  isStale={previewSnapshot?.stale ?? false}
                  onUpdateWeightIndicator={updateWeightIndicator}
                  previewSnapshot={previewSnapshot}
                  weightIndicators={weightIndicators}
                />
              ) : activeStep === "simulation" ? (
                <SimulationPositionPanel
                  appliedWeightIndicators={previewAppliedWeightIndicators}
                  backtestValidationError={backtestValidationError}
                  conditionGroups={conditionGroups}
                  executionDraft={backtestExecutionDraft}
                  isBacktestValidationPending={isBacktestValidationPending}
                  isMarketTemplateError={defaultMarketTemplateQuery.isError}
                  isMarketTemplateLoading={defaultMarketTemplateQuery.isLoading}
                  marketTemplateError={defaultMarketTemplateQuery.error}
                  onRetryMarketTemplate={() => {
                    void defaultMarketTemplateQuery.refetch()
                  }}
                  previewSnapshot={previewSnapshot}
                  settings={effectiveSimulationSettings}
                  onSettingsChange={handleSimulationSettingsChange}
                />
              ) : activeStep === "backtest" ? (
                <BacktestPanel
                  benchmark={backtestBenchmark}
                  draft={backtestExecutionDraft}
                  period={backtestPeriod}
                  onBenchmarkChange={setBacktestBenchmark}
                  onPeriodChange={setBacktestPeriod}
                />
              ) : null}
            </div>

            {showStepActions ? (
              <>
                <Separator className={cn(!isSplitStep && "mt-5")} />

                <div
                  className={cn(
                    "shrink-0 bg-background pt-4",
                    isSplitStep
                      ? cn(
                          "grid grid-cols-1 gap-y-2",
                          splitStepLayoutClassName,
                          "xl:gap-x-0"
                        )
                      : "flex items-center gap-2"
                  )}
                >
                  {activeStep === "indicators" ? (
                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        variant="default"
                        size="lg"
                        className="w-full bg-foreground text-background hover:bg-foreground/90 sm:w-48"
                        disabled={!canEditConditions}
                        onClick={() => setActiveStep("weights")}
                        type="button"
                      >
                        配置权重
                      </Button>
                    </div>
                  ) : activeStep === "weights" ? (
                    <Button
                      variant="default"
                      size="lg"
                      className="w-full sm:w-48"
                      disabled={
                        isPreviewPending ||
                        !canEditWeights ||
                        !hasRealMetricsCatalog
                      }
                      onClick={() => void openPreview()}
                      type="button"
                    >
                      {isPreviewPending ? (
                        <Spinner data-icon="inline-start" />
                      ) : null}
                      股池预览
                    </Button>
                  ) : activeStep === "preview" ? (
                    <>
                      <div className="flex flex-wrap items-center gap-2">
                        <Button
                          variant="default"
                          size="lg"
                          className="w-full sm:w-48"
                          disabled={!canEnterSimulation}
                          onClick={() => setActiveStep("simulation")}
                          type="button"
                        >
                          模拟建仓
                        </Button>
                      </div>
                      <div className="hidden xl:block" />
                      <Button
                        variant="outline"
                        size="lg"
                        className="w-full sm:w-48 xl:ml-2"
                        disabled={isPreviewPending}
                        onClick={() => void openPreview()}
                        type="button"
                      >
                        {isPreviewPending ? (
                          <Spinner data-icon="inline-start" />
                        ) : null}
                        更新股池
                      </Button>
                    </>
                  ) : activeStep === "simulation" ? (
                    <>
                      <div className="flex flex-wrap items-center gap-2">
                        <Button
                          variant="default"
                          size="lg"
                          className="w-full sm:w-48"
                          disabled={!canEnterBacktest}
                          onClick={() => setActiveStep("backtest")}
                          type="button"
                        >
                          {isBacktestValidationPending ? (
                            <Spinner data-icon="inline-start" />
                          ) : (
                            <ArrowRight data-icon="inline-start" />
                          )}
                          进入回测
                        </Button>
                      </div>
                      <div className="hidden xl:block" />
                      <div className="hidden xl:block" />
                    </>
                  ) : null}
                </div>
              </>
            ) : null}

            {activeStep === "backtest" ? (
              <>
                <Separator />

                <div
                  className={cn(
                    "grid shrink-0 grid-cols-1 gap-y-2 bg-background pt-4",
                    splitStepLayoutClassName,
                    "xl:gap-x-0"
                  )}
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      variant="default"
                      size="lg"
                      className="w-full sm:w-48"
                      disabled
                      type="button"
                    >
                      运行策略待接入
                    </Button>
                  </div>
                  <div className="hidden xl:block" />
                  <div className="hidden xl:block" />
                </div>
              </>
            ) : null}
          </div>
        </main>
      </div>
    </section>
  )
}
