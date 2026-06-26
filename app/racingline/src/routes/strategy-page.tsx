import {
  Fragment,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react"
import { useNavigate } from "react-router-dom"
import { createChart, LineSeries } from "lightweight-charts"
import { useQueryClient } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import {
  useDefaultMarketFeeTemplateQuery,
  useMetricsQuery,
  useStrategyBacktestCreateMutation,
  useStrategyBacktestNavQuery,
  useStrategyBacktestOptionsQuery,
  useStrategyBacktestPerformanceQuery,
  useStrategyBacktestOverviewUiQuery,
  useStrategyBacktestRebalanceRecordsUiQuery,
  useStrategyBacktestStatusQuery,
  useStrategyBacktestValidateQuery,
  useStrategyPortfolioCreateMutation,
  useStrategyPreviewOpenMutation,
} from "@/api/hooks"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty"
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  buildStrategyMetricCatalog,
  buildStrategyPreviewRuleSpec,
  buildStrategyScoringCatalog,
  StrategyRuleSpecError,
} from "@/features/strategy/adapters"
import {
  acceptStrategyBacktestRunForStep5,
  hasStrategyBacktestConfigChanged,
  isStrategyBacktestFailedStatus,
  isStrategyBacktestResultReady,
  isStrategyBacktestTerminalStatus,
  mergeStrategyBacktestStatus,
} from "@/features/strategy/backtest"
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
  formatComparableIndicator,
  formatWeightIndicator,
} from "@/features/strategy/utils"
import { cn } from "@/lib/utils"
import type {
  StrategyBacktestNavPoint,
  StrategyBacktestPerformanceUiView,
  StrategyBacktestPerformanceView,
  StrategyBacktestCreateRequest,
  StrategyBacktestRebalanceRecordSummary as ApiBacktestRebalanceRecordSummary,
  StrategyBacktestRebalanceUiRow as ApiBacktestRebalanceUiRow,
  StrategyBacktestRunRecord,
  StrategyBacktestRunStatus,
  StrategyBacktestRunStatusView,
  StrategyBacktestValidateRequest,
} from "@/types/rearview"
import { Play } from "lucide-react"

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
  buyTopN: 5,
  maxPositions: 5,
  singlePositionLimitPercent: 10,
  transactionFees: {
    stampDutyRatePercent: 0.05,
    transferFeeRatePercent: 0.001,
    commissionRatePercent: 0.01,
    slippageRatePercent: 0.1,
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
const toastAnimationMs = 180
const toastVisibleMs = 2_000
const toastLeaveDelayMs = toastVisibleMs - toastAnimationMs

function createPreviewTimingLogger() {
  const enabled =
    import.meta.env.DEV &&
    import.meta.env.VITE_RACINGLINE_PREVIEW_TIMING === "1"
  const startedAt = performance.now()
  const marks: { elapsed_ms: number; label: string }[] = []

  return {
    flush() {
      if (enabled) {
        console.debug("[racingline-preview-timing]", marks)
      }
    },
    mark(label: string) {
      if (enabled) {
        marks.push({
          elapsed_ms: Math.round(performance.now() - startedAt),
          label,
        })
      }
    },
  }
}

type BacktestPeriod = BacktestPeriodValue
type BacktestBenchmark =
  (typeof backtestBenchmarkOptions)[number]["securityCode"]
type BacktestNetValuePoint = {
  benchmark: number | null
  strategy: number
  time: string
}
type BacktestTradeDirection = "buy" | "hold" | "sell"
type BacktestRebalanceTrade = {
  changePercent: string
  contribution: string
  costPrice: string
  currentPrice: string
  direction: BacktestTradeDirection
  holdingDays: string
  securityCode: string
  securityName: string
}
type BacktestRebalanceRecord = {
  buyCount: number
  date: string
  holdCount: number
  positionCount: number
  sellCount: number
  trades: BacktestRebalanceTrade[]
}
type BacktestPerformanceGroup = {
  metrics: { label: string; value: string }[]
  title: string
}

function buildPreviewWeightIndicators(weightIndicators: WeightIndicator[]) {
  const source =
    weightIndicators.length > 0
      ? weightIndicators
      : defaultPreviewWeightIndicators

  return source.map(cloneWeightIndicator)
}

function cloneWeightIndicators(weightIndicators: WeightIndicator[]) {
  return weightIndicators.map(cloneWeightIndicator)
}

function cloneWeightIndicator(indicator: WeightIndicator): WeightIndicator {
  return {
    ...indicator,
    extraConditions: indicator.extraConditions?.map((condition) => ({
      ...condition,
    })),
  }
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

function buildStrategyBacktestCreateRequest({
  backtestExecutionDraft,
  benchmark,
  period,
  previewSnapshot,
  rangeHint,
  selectedBenchmarkLabel,
  selectedPeriodDescription,
  selectedPeriodLabel,
  settings,
}: {
  backtestExecutionDraft: BacktestExecutionDraft
  benchmark: BacktestBenchmark
  period: BacktestPeriod
  previewSnapshot: PreviewSnapshot
  rangeHint?: { end_date: string; start_date: string } | null
  selectedBenchmarkLabel: string
  selectedPeriodDescription: string | null
  selectedPeriodLabel: string
  settings: SimulationSettings
}): StrategyBacktestCreateRequest {
  return {
    ...buildBacktestExecutionRequestDraft({
      benchmark,
      draft: backtestExecutionDraft,
      period,
      rangeHint,
    }),
    client_request_id: createId("strategy-backtest"),
    preview_id: previewSnapshot.previewId,
    preview_range: {
      end_date: previewSnapshot.range.endDate,
      start_date: previewSnapshot.range.startDate,
    },
    ui_display_snapshot: {
      benchmark: {
        label: selectedBenchmarkLabel,
        security_code: benchmark,
      },
      period: {
        key: period,
        label: selectedPeriodLabel,
        resolved_range: selectedPeriodDescription,
      },
      preview: {
        created_at: previewSnapshot.createdAt,
        preview_id: previewSnapshot.previewId,
        selected_trade_date: previewSnapshot.range.selectedTradeDate ?? null,
      },
      simulation: {
        buy_signal_top_n: settings.buyTopN,
        initial_capital: settings.initialCapital,
        max_positions: settings.maxPositions,
        single_position_limit_pct: settings.singlePositionLimitPercent,
        stop_loss_rule_count:
          backtestExecutionDraft.summary.enabled_exit_rule_count,
      },
    },
  }
}

function BacktestPanel({
  activeRun,
  backtestExecutionDraft,
  backtestValidationError,
  benchmark,
  isBacktestValidationPending,
  isMarketTemplateError,
  isMarketTemplateLoading,
  onBenchmarkChange,
  onPeriodChange,
  onRunChange,
  period,
  previewSnapshot,
  settings,
}: {
  activeRun: StrategyBacktestRunRecord | null
  backtestExecutionDraft: BacktestExecutionDraft | null
  backtestValidationError: string | null
  benchmark: BacktestBenchmark
  isBacktestValidationPending: boolean
  isMarketTemplateError: boolean
  isMarketTemplateLoading: boolean
  onBenchmarkChange: (benchmark: BacktestBenchmark) => void
  onPeriodChange: (period: BacktestPeriod) => void
  onRunChange: (run: StrategyBacktestRunRecord) => void
  period: BacktestPeriod
  previewSnapshot: PreviewSnapshot | null
  settings: SimulationSettings
}) {
  const queryClient = useQueryClient()
  const activeRunId = activeRun?.strategy_backtest_run_id ?? null
  const [rebalanceSelection, setRebalanceSelection] = useState<{
    date: string | null
    runId: string | null
  }>({ date: null, runId: null })
  const rebalanceDateScrollerRef = useRef<HTMLDivElement | null>(null)
  const optionsQuery = useStrategyBacktestOptionsQuery(benchmark)
  const createBacktestMutation = useStrategyBacktestCreateMutation()
  const statusQuery = useStrategyBacktestStatusQuery(activeRunId)
  const currentRun = statusQuery.data ?? activeRun
  const hasPendingConfigChange = hasStrategyBacktestConfigChanged(
    currentRun,
    backtestExecutionDraft,
    period,
    benchmark
  )
  const isRunInProgress = Boolean(
    currentRun && !isStrategyBacktestTerminalStatus(currentRun.status)
  )
  const isResultReady = isStrategyBacktestResultReady(
    currentRun,
    backtestExecutionDraft,
    period,
    benchmark
  )
  const selectedRebalanceDate =
    rebalanceSelection.runId === activeRunId ? rebalanceSelection.date : null
  const setSelectedRebalanceDate = useCallback(
    (date: string | null) => setRebalanceSelection({ date, runId: activeRunId }),
    [activeRunId]
  )
  const overviewQuery = useStrategyBacktestOverviewUiQuery(
    activeRunId,
    null,
    isResultReady
  )
  const overviewRebalance = overviewQuery.data?.rebalance ?? null
  const overviewSelectedRebalanceDate =
    overviewRebalance?.selected_trade_date ?? null
  const effectiveSelectedRebalanceDate =
    selectedRebalanceDate ?? overviewSelectedRebalanceDate
  const shouldFetchSelectedRebalanceRows = Boolean(
    selectedRebalanceDate &&
      selectedRebalanceDate !== overviewSelectedRebalanceDate
  )
  const selectedRebalanceRowsQuery =
    useStrategyBacktestRebalanceRecordsUiQuery(
      activeRunId,
      selectedRebalanceDate,
      isResultReady && shouldFetchSelectedRebalanceRows
    )
  const selectedRebalanceRowsResponse =
    shouldFetchSelectedRebalanceRows &&
    selectedRebalanceRowsQuery.data?.selected_trade_date ===
      selectedRebalanceDate
      ? selectedRebalanceRowsQuery.data
      : shouldFetchSelectedRebalanceRows
        ? null
        : overviewRebalance
  const selectedRebalanceRowsError =
    shouldFetchSelectedRebalanceRows && selectedRebalanceRowsQuery.isError
      ? selectedRebalanceRowsQuery.error
      : null

  const periodOptions = useMemo(
    () =>
      optionsQuery.data
        ? optionsQuery.data.period_options.map((option) => ({
            description: `${option.resolved_start_date} - ${option.resolved_end_date}`,
            label: option.label,
            value: option.period_key as BacktestPeriod,
          }))
        : backtestPeriodOptions.map((option) => ({
            description: "等待后端解析动态区间",
            label: option.label,
            value: option.value,
          })),
    [optionsQuery.data]
  )
  const benchmarkOptions = useMemo(
    () =>
      optionsQuery.data
        ? optionsQuery.data.benchmark_options.map((option) => ({
            availabilityStatus: option.availability_status,
            label: option.label,
            securityCode: option.security_code as BacktestBenchmark,
          }))
        : backtestBenchmarkOptions.map((option) => ({
            availabilityStatus: "available",
            label: option.label,
            securityCode: option.securityCode,
          })),
    [optionsQuery.data]
  )
  const selectedPeriodOption =
    periodOptions.find((option) => option.value === period) ?? periodOptions[0]
  const selectedPeriodApiOption = optionsQuery.data?.period_options.find(
    (option) => option.period_key === period
  )
  const selectedBenchmarkOption =
    benchmarkOptions.find((option) => option.securityCode === benchmark) ??
    benchmarkOptions[0]
  const selectedPeriodLabel = selectedPeriodOption?.label ?? period
  const selectedBenchmarkLabel =
    selectedBenchmarkOption?.label ??
    backtestBenchmarkOptions.find((option) => option.securityCode === benchmark)
      ?.label ??
    benchmark
  const navPoints = useMemo(
    () => mapStrategyBacktestNavPoints(overviewQuery.data?.nav_points ?? []),
    [overviewQuery.data?.nav_points]
  )
  const latestNetValuePoint = navPoints.at(-1)
  const latestExcessReturn =
    latestNetValuePoint && latestNetValuePoint.benchmark !== null
      ? formatSignedPercent(
          latestNetValuePoint.strategy - latestNetValuePoint.benchmark
        )
      : ""
  const rebalanceRecords = useMemo(
    () =>
      mapApiBacktestRebalanceRecordSummaries(
        overviewRebalance?.records ?? []
      ),
    [overviewRebalance?.records]
  )
  const selectedRebalanceRecordSummary =
    rebalanceRecords.find(
      (record) => record.date === effectiveSelectedRebalanceDate
    ) ??
    rebalanceRecords.at(-1)
  const selectedRebalanceRows = useMemo(
    () => {
      if (
        !selectedRebalanceRowsResponse ||
        selectedRebalanceRowsResponse.selected_trade_date !==
          selectedRebalanceRecordSummary?.date
      ) {
        return []
      }

      return mapApiBacktestRebalanceUiRows(
        selectedRebalanceRowsResponse.selected_rows
      )
    },
    [selectedRebalanceRecordSummary?.date, selectedRebalanceRowsResponse]
  )
  const selectedRebalanceRecord = selectedRebalanceRecordSummary
    ? {
        ...selectedRebalanceRecordSummary,
        trades: selectedRebalanceRows,
      }
    : null
  const selectedRebalanceTradeSections = selectedRebalanceRecord
    ? buildRebalanceTradeSections(selectedRebalanceRecord.trades)
    : []
  const selectedRebalanceRecordDate = selectedRebalanceRecord?.date ?? null
  const isSelectedRebalanceRowsLoading = Boolean(
    shouldFetchSelectedRebalanceRows &&
      selectedRebalanceRowsQuery.isFetching &&
      !selectedRebalanceRowsResponse
  )

  useEffect(() => {
    const scroller = rebalanceDateScrollerRef.current

    if (!scroller || !selectedRebalanceRecordDate) {
      return
    }

    const frameId = window.requestAnimationFrame(() => {
      const selectedButton = scroller.querySelector<HTMLElement>(
        `[data-rebalance-date="${selectedRebalanceRecordDate}"]`
      )

      selectedButton?.scrollIntoView({
        behavior: "auto",
        block: "nearest",
        inline: selectedRebalanceDate ? "nearest" : "end",
      })
    })

    return () => window.cancelAnimationFrame(frameId)
  }, [rebalanceRecords.length, selectedRebalanceDate, selectedRebalanceRecordDate])
  const performanceGroups = useMemo(
    () =>
      buildBacktestPerformanceGroups(
        overviewQuery.data?.performance ?? null,
        latestExcessReturn
      ),
    [latestExcessReturn, overviewQuery.data?.performance]
  )
  const actionDisabled = Boolean(
    !previewSnapshot ||
      previewSnapshot.stale ||
      !backtestExecutionDraft ||
      backtestValidationError ||
      isBacktestValidationPending ||
      isMarketTemplateLoading ||
      isMarketTemplateError ||
      optionsQuery.isLoading ||
      optionsQuery.isError ||
      selectedBenchmarkOption?.availabilityStatus === "unavailable" ||
      createBacktestMutation.isPending ||
      isRunInProgress
  )
  const actionLabel = createBacktestMutation.isPending
    ? "提交中"
    : isRunInProgress
      ? "回测中"
      : activeRunId
        ? "重新回测"
        : "开始回测"
  const showActionSpinner =
    createBacktestMutation.isPending ||
    isRunInProgress ||
    isBacktestValidationPending ||
    optionsQuery.isLoading
  useEffect(() => {
    if (activeRun && statusQuery.data) {
      const mergedRun = mergeStrategyBacktestStatus(activeRun, statusQuery.data)
      if (mergedRun !== activeRun) {
        onRunChange(mergedRun)
      }
    }
  }, [activeRun, onRunChange, statusQuery.data])

  const runBacktest = useCallback(async () => {
    if (!backtestExecutionDraft || !previewSnapshot) {
      return
    }

    const request = buildStrategyBacktestCreateRequest({
      backtestExecutionDraft,
      benchmark,
      period,
      previewSnapshot,
      rangeHint: selectedPeriodApiOption
        ? {
            end_date: selectedPeriodApiOption.resolved_end_date,
            start_date: selectedPeriodApiOption.resolved_start_date,
          }
        : null,
      selectedBenchmarkLabel,
      selectedPeriodDescription: selectedPeriodOption?.description ?? null,
      selectedPeriodLabel,
      settings,
    })
    const run = await createBacktestMutation.mutateAsync(request)
    queryClient.setQueryData(
      queryKeys.strategyBacktest(run.strategy_backtest_run_id),
      run
    )
    onRunChange(run)
  }, [
    backtestExecutionDraft,
    benchmark,
    createBacktestMutation,
    onRunChange,
    period,
    previewSnapshot,
    queryClient,
    selectedBenchmarkLabel,
    selectedPeriodLabel,
    selectedPeriodOption?.description,
    selectedPeriodApiOption,
    settings,
  ])

  return (
    <StrategySplitPanel
      main={
        <div className="flex w-full flex-col gap-4">
          <div className="text-sm font-medium">回测配置</div>
          <FieldGroup className="grid gap-3 md:grid-cols-[minmax(8rem,0.8fr)_minmax(12rem,1fr)_minmax(8rem,0.72fr)] md:items-end xl:pr-4">
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
                    {periodOptions.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        <div className="flex flex-col gap-0.5">
                          <span>{option.label}</span>
                          <span className="text-xs text-muted-foreground">
                            {option.description}
                          </span>
                        </div>
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
                    {benchmarkOptions.map((option) => (
                      <SelectItem
                        key={option.securityCode}
                        disabled={option.availabilityStatus === "unavailable"}
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
              disabled={actionDisabled}
              onClick={() => void runBacktest()}
              variant="outline"
              size="lg"
              type="button"
            >
              {showActionSpinner ? <Spinner data-icon="inline-start" /> : null}
              {actionLabel}
            </Button>
          </FieldGroup>

          <BacktestStatusAlert
            backtestValidationError={backtestValidationError}
            createError={createBacktestMutation.error}
            isMarketTemplateError={isMarketTemplateError}
            optionsError={optionsQuery.error}
            run={currentRun}
            runError={statusQuery.error}
          />
          <BacktestToastViewport
            hasPendingConfigChange={hasPendingConfigChange}
            run={currentRun}
          />

          <section className="flex flex-col gap-3 xl:pr-4">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-medium">净值走势</div>
              <div className="flex items-center gap-4 text-xs text-muted-foreground">
                <div className="inline-flex items-center gap-1.5">
                  <span className="size-2 bg-foreground" />
                  策略净值
                </div>
                <div className="inline-flex items-center gap-1.5">
                  <span className="size-2 bg-muted-foreground" />
                  基准净值
                </div>
              </div>
            </div>
            {overviewQuery.isLoading ? (
              <Skeleton className="h-60 w-full" />
            ) : navPoints.length > 0 ? (
              <BacktestNetValueChart points={navPoints} />
            ) : (
              <Empty className="h-60 border">
                <EmptyHeader>
                  <EmptyTitle>暂无净值数据</EmptyTitle>
                  <EmptyDescription>
                    提交回测并等待计算完成后展示策略与基准净值。
                  </EmptyDescription>
                </EmptyHeader>
              </Empty>
            )}
          </section>

          <Separator className="bg-border/60" />

          <section className="flex flex-col gap-3 xl:pr-4">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-medium">持仓记录</div>
              <div className="text-xs text-muted-foreground tabular-nums">
                {rebalanceRecords.length} 个调仓日
              </div>
            </div>

            {overviewQuery.isLoading ? (
              <Skeleton className="h-36 w-full" />
            ) : rebalanceRecords.length > 0 ? (
              <>
                <div
                  ref={rebalanceDateScrollerRef}
                  className="h-[32px] shrink-0 overflow-x-auto overflow-y-hidden overscroll-x-contain pb-3 [scrollbar-width:thin] [&::-webkit-scrollbar]:h-[2px] [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent"
                >
                  <div className="flex min-w-max gap-1.5 pr-1">
                    {rebalanceRecords.map((record) => {
                      const isSelected =
                        record.date === selectedRebalanceRecord?.date

                      return (
                        <Button
                          key={record.date}
                          aria-label={`${record.date} 持仓 ${record.positionCount} 只`}
                          aria-pressed={isSelected}
                          data-rebalance-date={record.date}
                          data-state={isSelected ? "selected" : "idle"}
                          className="grid h-[18px] w-[6.25rem] shrink-0 grid-cols-[2.75rem_2rem] items-center justify-center gap-1 px-1 text-muted-foreground hover:bg-muted data-[state=selected]:bg-muted data-[state=selected]:text-foreground"
                          size="sm"
                          type="button"
                          variant="ghost"
                          onClick={() => setSelectedRebalanceDate(record.date)}
                        >
                          <span className="text-right text-[11px] leading-none tabular-nums">
                            {formatCompactDate(record.date)}
                          </span>
                          <span className="text-left text-[11px] leading-none font-normal tabular-nums opacity-80">
                            {record.positionCount}只
                          </span>
                        </Button>
                      )
                    })}
                  </div>
                </div>

                {selectedRebalanceRecord ? (
                  <div className="flex flex-col gap-2">
                    <div className="flex items-end justify-between gap-3">
                      <div className="text-sm font-medium tabular-nums">
                        {selectedRebalanceRecord.date}
                      </div>
                      <div className="text-xs text-muted-foreground tabular-nums">
                        调入 {selectedRebalanceRecord.buyCount} 只 / 持有{" "}
                        {selectedRebalanceRecord.holdCount} 只 / 卖出{" "}
                        {selectedRebalanceRecord.sellCount} 只
                      </div>
                    </div>
                    {selectedRebalanceRowsError ? (
                      <Alert variant="destructive">
                        <AlertTitle>持仓明细加载失败</AlertTitle>
                        <AlertDescription>
                          {formatErrorMessage(selectedRebalanceRowsError)}
                        </AlertDescription>
                      </Alert>
                    ) : isSelectedRebalanceRowsLoading ? (
                      <Skeleton className="min-h-32 w-full" />
                    ) : selectedRebalanceTradeSections.some(
                      (section) => section.trades.length > 0
                    ) ? (
                      <Table className="w-full table-fixed text-xs leading-snug [&_td]:overflow-hidden [&_th]:overflow-hidden">
                        <TableHeader>
                          <TableRow className="hover:bg-transparent">
                            <TableHead className="h-6 w-[20%] px-1">
                              股票
                            </TableHead>
                            <TableHead className="h-6 w-[13%] px-1 text-right">
                              持仓天数
                            </TableHead>
                            <TableHead className="h-6 w-[15%] px-1 text-right">
                              涨跌幅
                            </TableHead>
                            <TableHead className="h-6 w-[17%] px-1 text-right">
                              成本价
                            </TableHead>
                            <TableHead className="h-6 w-[17%] px-1 text-right">
                              现价
                            </TableHead>
                            <TableHead className="h-6 w-[18%] px-1 text-right">
                              收益贡献
                            </TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {selectedRebalanceTradeSections.map((section) => (
                            <Fragment key={section.direction}>
                              <TableRow className="bg-muted/30 hover:bg-muted/30">
                                <TableCell
                                  colSpan={6}
                                  className="px-1 py-1 text-xs font-medium text-muted-foreground"
                                >
                                  {formatTradeDirection(section.direction)}{" "}
                                  {section.trades.length} 只
                                </TableCell>
                              </TableRow>
                              {section.trades.map((trade, tradeIndex) => (
                                <TableRow
                                  key={`${selectedRebalanceRecord.date}-${trade.direction}-${trade.securityCode}-${tradeIndex}`}
                                >
                                  <TableCell className="px-1 py-0.5">
                                    <div className="grid min-w-0 grid-cols-[4.25em_minmax(0,1fr)] items-center gap-1">
                                      <span className="truncate font-medium">
                                        {trade.securityName}
                                      </span>
                                      <span className="truncate text-muted-foreground tabular-nums">
                                        {trade.securityCode}
                                      </span>
                                    </div>
                                  </TableCell>
                                  <TableCell className="px-1 py-0.5 text-right tabular-nums">
                                    {trade.holdingDays}
                                  </TableCell>
                                  <TableCell
                                    className={cn(
                                      "px-1 py-0.5 text-right font-medium tabular-nums",
                                      getSignedValueClassName(
                                        trade.changePercent
                                      )
                                    )}
                                  >
                                    {trade.changePercent}
                                  </TableCell>
                                  <TableCell className="px-1 py-0.5 text-right tabular-nums">
                                    {trade.costPrice}
                                  </TableCell>
                                  <TableCell className="px-1 py-0.5 text-right tabular-nums">
                                    {trade.currentPrice}
                                  </TableCell>
                                  <TableCell
                                    className={cn(
                                      "px-1 py-0.5 text-right font-medium tabular-nums",
                                      getSignedValueClassName(
                                        trade.contribution
                                      )
                                    )}
                                  >
                                    {trade.contribution}
                                  </TableCell>
                                </TableRow>
                              ))}
                            </Fragment>
                          ))}
                        </TableBody>
                      </Table>
                    ) : (
                      <Empty className="min-h-32 border">
                        <EmptyHeader>
                          <EmptyTitle>该日无持仓明细</EmptyTitle>
                          <EmptyDescription>
                            当前日期没有调入、持有或卖出记录。
                          </EmptyDescription>
                        </EmptyHeader>
                      </Empty>
                    )}
                  </div>
                ) : null}
              </>
            ) : (
              <Empty className="min-h-36 border">
                <EmptyHeader>
                  <EmptyTitle>暂无持仓记录</EmptyTitle>
                  <EmptyDescription>
                    回测结果写入后会展示调仓日和对应交易明细。
                  </EmptyDescription>
                </EmptyHeader>
              </Empty>
            )}
          </section>

        </div>
      }
      aside={
        <Card className="h-fit bg-transparent py-0 ring-0">
          <CardHeader>
            <div className="inline-flex items-baseline gap-2">
              <CardTitle>策略业绩</CardTitle>
              {latestNetValuePoint ? (
                <div className="text-xs text-muted-foreground tabular-nums">
                  {latestNetValuePoint.time}
                </div>
              ) : null}
            </div>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            {latestNetValuePoint ? (
              <>
                <div className="grid grid-cols-2 gap-3">
                  <BacktestSummaryMetric
                    label="策略净值"
                    value={formatNetValue(latestNetValuePoint.strategy)}
                  />
                  <BacktestSummaryMetric
                    label="基准净值"
                    value={
                      latestNetValuePoint.benchmark === null
                        ? "—"
                        : formatNetValue(latestNetValuePoint.benchmark)
                    }
                  />
                </div>

                <Separator />
              </>
            ) : null}

            <div className="flex flex-col gap-4">
              {performanceGroups.map((group) => (
                <div key={group.title} className="flex min-w-0 flex-col gap-2">
                  <div className="text-xs font-medium text-muted-foreground">
                    {group.title}
                  </div>
                  <div className="flex flex-col gap-1.5">
                    {group.metrics.map((metric) => (
                      <div
                        key={metric.label}
                        className="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3"
                      >
                        <div className="truncate text-xs text-muted-foreground">
                          {metric.label}
                        </div>
                        <div
                          className={cn(
                            "text-sm font-medium tabular-nums",
                            getBacktestPerformanceToneClassName(
                              metric.label,
                              metric.value
                            )
                          )}
                        >
                          {metric.value}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
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

function getSignedValueClassName(value: string) {
  if (value.startsWith("+")) {
    return "text-[color:var(--portfolio-up)]"
  }

  if (value.startsWith("-")) {
    return "text-[color:var(--portfolio-down)]"
  }

  return "text-foreground"
}

function getBacktestPerformanceToneClassName(label: string, value: string) {
  if (value === "—") {
    return "text-foreground"
  }

  if (
    label === "持仓收益" ||
    label === "年化收益" ||
    label === "Alpha" ||
    label === "超额收益"
  ) {
    return getSignedValueClassName(value)
  }

  if (label === "最大回撤") {
    return getSignedValueClassName(value)
  }

  return "text-foreground"
}

function formatTradeDirection(direction: BacktestTradeDirection) {
  if (direction === "buy") {
    return "调入"
  }

  if (direction === "hold") {
    return "持有"
  }

  return "卖出"
}

function buildRebalanceTradeSections(trades: BacktestRebalanceTrade[]) {
  return (["buy", "hold", "sell"] as const).map((direction) => ({
    direction,
    trades: trades.filter((trade) => trade.direction === direction),
  }))
}

function BacktestStatusAlert({
  backtestValidationError,
  createError,
  isMarketTemplateError,
  optionsError,
  run,
  runError,
}: {
  backtestValidationError: string | null
  createError: unknown
  isMarketTemplateError: boolean
  optionsError: unknown
  run: StrategyBacktestRunStatusView | StrategyBacktestRunRecord | null
  runError: unknown
}) {
  if (isMarketTemplateError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>市场费率模板不可用</AlertTitle>
        <AlertDescription>无法生成 Step5 回测执行配置。</AlertDescription>
      </Alert>
    )
  }

  if (backtestValidationError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>回测配置未通过校验</AlertTitle>
        <AlertDescription>{backtestValidationError}</AlertDescription>
      </Alert>
    )
  }

  if (optionsError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>回测选项加载失败</AlertTitle>
        <AlertDescription>{formatErrorMessage(optionsError)}</AlertDescription>
      </Alert>
    )
  }

  if (createError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>回测任务提交失败</AlertTitle>
        <AlertDescription>{formatErrorMessage(createError)}</AlertDescription>
      </Alert>
    )
  }

  if (runError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>回测任务状态加载失败</AlertTitle>
        <AlertDescription>{formatErrorMessage(runError)}</AlertDescription>
      </Alert>
    )
  }

  if (run && isStrategyBacktestFailedStatus(run.status)) {
    return (
      <Alert variant="destructive">
        <AlertTitle>{getStrategyBacktestStatusLabel(run.status)}</AlertTitle>
        <AlertDescription>
          {run.error_message || "后端未返回失败原因。"}
        </AlertDescription>
      </Alert>
    )
  }

  return null
}

function BacktestToastViewport({
  hasPendingConfigChange,
  run,
}: {
  hasPendingConfigChange: boolean
  run: StrategyBacktestRunStatusView | StrategyBacktestRunRecord | null
}) {
  if (!hasPendingConfigChange && !run) {
    return null
  }

  return (
    <div className="pointer-events-none fixed top-[4.5rem] right-4 left-4 z-50 flex w-auto flex-col gap-2 sm:left-auto sm:w-96">
      {hasPendingConfigChange ? <PendingConfigChangeToast /> : null}
      <BacktestRunStatusToast
        key={run?.strategy_backtest_run_id ?? "empty"}
        run={run}
      />
    </div>
  )
}

function PendingConfigChangeToast() {
  const [visible, setVisible] = useState(true)
  const [isLeaving, setIsLeaving] = useState(false)

  useEffect(() => {
    const leaveTimeoutId = window.setTimeout(
      () => setIsLeaving(true),
      toastLeaveDelayMs
    )
    const removeTimeoutId = window.setTimeout(
      () => setVisible(false),
      toastVisibleMs
    )

    return () => {
      window.clearTimeout(leaveTimeoutId)
      window.clearTimeout(removeTimeoutId)
    }
  }, [])

  if (!visible) {
    return null
  }

  return (
    <Alert
      className={cn(
        "pointer-events-auto shadow-lg",
        isLeaving ? "racingline-toast-leave" : "racingline-toast-enter"
      )}
    >
      <AlertTitle>配置已变更</AlertTitle>
      <AlertDescription>
        当前展示结果不再匹配所选周期、基准或策略配置，需要重新回测。
      </AlertDescription>
    </Alert>
  )
}

function BacktestRunStatusToast({
  run,
}: {
  run: StrategyBacktestRunStatusView | StrategyBacktestRunRecord | null
}) {
  const status = run?.status ?? null
  const runId = run?.strategy_backtest_run_id ?? null
  const isFailedStatus = status ? isStrategyBacktestFailedStatus(status) : false
  const isTerminal = status ? isStrategyBacktestTerminalStatus(status) : false
  const [visible, setVisible] = useState(true)
  const [isLeaving, setIsLeaving] = useState(false)

  useEffect(() => {
    if (!runId || isFailedStatus || !isTerminal) {
      return
    }

    const leaveTimeoutId = window.setTimeout(
      () => setIsLeaving(true),
      toastLeaveDelayMs
    )
    const removeTimeoutId = window.setTimeout(
      () => setVisible(false),
      toastVisibleMs
    )

    return () => {
      window.clearTimeout(leaveTimeoutId)
      window.clearTimeout(removeTimeoutId)
    }
  }, [isFailedStatus, isTerminal, runId])

  if (!run || isFailedStatus || !visible) {
    return null
  }

  return (
    <Alert
      className={cn(
        "pointer-events-auto shadow-lg",
        isLeaving ? "racingline-toast-leave" : "racingline-toast-enter"
      )}
    >
      <AlertTitle className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <span>
          {getStrategyBacktestStatusLabel(run.status)}
        </span>
        {!isTerminal ? <Spinner data-icon="inline-start" /> : null}
      </AlertTitle>
    </Alert>
  )
}

function getStrategyBacktestStatusLabel(status?: StrategyBacktestRunStatus) {
  const labels: Record<StrategyBacktestRunStatus, string> = {
    calculating_nav: "计算净值中",
    cancelled: "已取消",
    compiling_signals: "编译信号中",
    computing_performance: "计算业绩中",
    created: "已创建",
    failed_compile: "信号编译失败",
    failed_market_data: "行情数据失败",
    failed_simulation: "组合回测失败",
    failed_validation: "参数校验失败",
    failed_write: "结果写入失败",
    loading_market_data: "加载行情中",
    queued: "排队中",
    running_clickhouse: "执行信号查询中",
    succeeded: "回测完成",
    writing_results: "写入结果中",
  }

  return status ? labels[status] : "回测中"
}

function mapStrategyBacktestNavPoints(
  points: StrategyBacktestNavPoint[]
): BacktestNetValuePoint[] {
  return points.map((point) => ({
    benchmark:
      typeof point.benchmark_nav === "number" ? point.benchmark_nav : null,
    strategy: point.strategy_nav,
    time: point.trade_date,
  }))
}

function mapApiBacktestRebalanceRecordSummaries(
  records: ApiBacktestRebalanceRecordSummary[]
): BacktestRebalanceRecord[] {
  return records.map((record) => mapApiBacktestRebalanceRecordSummary(record, []))
}

function mapApiBacktestRebalanceRecordSummary(
  record: ApiBacktestRebalanceRecordSummary,
  rows: ApiBacktestRebalanceUiRow[]
): BacktestRebalanceRecord {
  return {
    buyCount: record.buy_count,
    date: record.trade_date,
    holdCount: record.hold_count,
    positionCount: record.position_count,
    sellCount: record.sell_count,
    trades: mapApiBacktestRebalanceUiRows(rows),
  }
}

function mapApiBacktestRebalanceUiRows(
  rows: ApiBacktestRebalanceUiRow[]
): BacktestRebalanceTrade[] {
  return rows.map((row) => ({
    changePercent: formatOptionalSignedPercent(row.change_pct),
    contribution: formatOptionalSignedPercent(row.contribution_pct),
    costPrice: formatOptionalCurrency(row.cost_price),
    currentPrice: formatOptionalCurrency(row.current_price),
    direction: row.direction,
    holdingDays:
      typeof row.holding_days === "number" ? `${row.holding_days}天` : "—",
    securityCode: row.security_code,
    securityName: row.security_name?.trim() || row.security_code,
  }))
}

function buildBacktestPerformanceGroups(
  performance:
    | StrategyBacktestPerformanceUiView
    | StrategyBacktestPerformanceView
    | null,
  latestExcessReturn: string
): BacktestPerformanceGroup[] {
  return [
    {
      title: "收益指标",
      metrics: [
        {
          label: "持仓收益",
          value: formatOptionalSignedPercent(
            readPerformanceMetric(performance, "holding_period_return")
          ),
        },
        {
          label: "年化收益",
          value: formatOptionalSignedPercent(
            readPerformanceMetric(performance, "annualized_return")
          ),
        },
        { label: "超额收益", value: latestExcessReturn || "—" },
        {
          label: "日胜率",
          value: formatOptionalPercent(performance?.daily_win_rate.value),
        },
      ],
    },
    {
      title: "风险指标",
      metrics: [
        {
          label: "最大回撤",
          value: formatOptionalDrawdown(
            readPerformanceMetric(performance, "max_drawdown")
          ),
        },
        {
          label: "年化波动率",
          value: formatOptionalPercent(
            readPerformanceMetric(performance, "annualized_volatility")
          ),
        },
        {
          label: "下行波动率",
          value: formatOptionalPercent(
            readPerformanceMetric(performance, "downside_deviation")
          ),
        },
      ],
    },
    {
      title: "性价比",
      metrics: [
        {
          label: "Sharpe Ratio",
          value: formatOptionalRatio(
            readPerformanceMetric(performance, "sharpe_ratio")
          ),
        },
        {
          label: "Sortino Ratio",
          value: formatOptionalRatio(
            readPerformanceMetric(performance, "sortino_ratio")
          ),
        },
        {
          label: "Calmar Ratio",
          value: formatOptionalRatio(
            readPerformanceMetric(performance, "calmar_ratio")
          ),
        },
        {
          label: "Treynor Ratio",
          value: formatOptionalRatio(
            readPerformanceMetric(performance, "treynor_ratio")
          ),
        },
      ],
    },
    {
      title: "相对市场",
      metrics: [
        {
          label: "Alpha",
          value: formatOptionalSignedPercent(
            readPerformanceMetric(performance, "alpha")
          ),
        },
        {
          label: "Beta",
          value: formatOptionalRatio(readPerformanceMetric(performance, "beta")),
        },
        {
          label: "Information Ratio",
          value: formatOptionalRatio(
            readPerformanceMetric(performance, "information_ratio")
          ),
        },
      ],
    },
  ]
}

function readPerformanceMetric(
  performance:
    | StrategyBacktestPerformanceUiView
    | StrategyBacktestPerformanceView
    | null,
  key: string
) {
  const value = performance?.metric[key]

  return typeof value === "number" && Number.isFinite(value) ? value : null
}

function formatCompactDate(date: string) {
  const [, month, day] = date.split("-")

  return `${month}/${day}`
}

function formatCurrency(value: number) {
  return `¥${value.toFixed(2)}`
}

function formatOptionalCurrency(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? formatCurrency(value)
    : "—"
}

function formatUiPercent(value: number) {
  return `${Number(value.toFixed(3))}%`
}

function formatNetValue(value: number) {
  return value.toFixed(4)
}

function formatSignedPercent(value: number) {
  const sign = value > 0 ? "+" : ""

  return `${sign}${(value * 100).toFixed(2)}%`
}

function formatOptionalSignedPercent(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? formatSignedPercent(value)
    : "—"
}

function formatOptionalPercent(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? `${(value * 100).toFixed(2)}%`
    : "—"
}

function formatOptionalDrawdown(value: number | null | undefined) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return "—"
  }

  if (value === 0) {
    return "0.00%"
  }

  return `-${(Math.abs(value) * 100).toFixed(2)}%`
}

function formatOptionalRatio(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? value.toFixed(2)
    : "—"
}

function formatErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message
  }

  return String(error || "Unknown error")
}

function BacktestNetValueChart({
  points,
}: {
  points: readonly BacktestNetValuePoint[]
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const container = containerRef.current

    if (!container || points.length === 0) {
      return
    }

    const chart = createChart(container, {
      width: container.clientWidth,
      height: 240,
      layout: {
        background: { color: "transparent" },
        textColor: "rgba(99, 95, 89, 0.78)",
        attributionLogo: false,
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { color: "rgba(120, 114, 108, 0.12)" },
      },
      crosshair: {
        vertLine: { color: "rgba(120, 114, 108, 0.20)" },
        horzLine: { color: "rgba(120, 114, 108, 0.20)" },
      },
      rightPriceScale: {
        borderVisible: false,
      },
      leftPriceScale: {
        visible: false,
      },
      timeScale: {
        borderVisible: false,
        timeVisible: false,
      },
      handleScroll: false,
      handleScale: false,
    })

    const strategySeries = chart.addSeries(LineSeries, {
      color: "#2b2622",
      lineWidth: 2,
      lastValueVisible: false,
      priceLineVisible: false,
    })

    const benchmarkSeries = chart.addSeries(LineSeries, {
      color: "#8e867e",
      lineWidth: 2,
      lastValueVisible: false,
      priceLineVisible: false,
    })

    strategySeries.setData(
      points.map((point) => ({
        time: point.time,
        value: point.strategy,
      }))
    )
    benchmarkSeries.setData(
      points
        .filter((point) => typeof point.benchmark === "number")
        .map((point) => ({
          time: point.time,
          value: point.benchmark as number,
        }))
    )
    chart.timeScale().fitContent()

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0]

      if (!entry) {
        return
      }

      chart.applyOptions({
        width: entry.contentRect.width,
      })
    })

    resizeObserver.observe(container)

    return () => {
      resizeObserver.disconnect()
      chart.remove()
    }
  }, [points])

  return <div ref={containerRef} className="h-60 w-full bg-muted/10" />
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
  const previewOpenMutation = useStrategyPreviewOpenMutation()
  const initialBacktestMutation = useStrategyBacktestCreateMutation()
  const createPortfolioMutation = useStrategyPortfolioCreateMutation()
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
  const strategyStopLossCatalogOptions =
    strategyCatalogOptions.length > 0
      ? strategyCatalogOptions
      : strategyScoringCatalog
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
  const [activeBacktestRun, setActiveBacktestRun] =
    useState<StrategyBacktestRunRecord | null>(null)
  const [backtestLaunchError, setBacktestLaunchError] = useState<string | null>(
    null
  )
  const [publishDialogOpen, setPublishDialogOpen] = useState(false)
  const [portfolioName, setPortfolioName] = useState("")
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
  const commissionRateMaxPercent =
    defaultMarketTemplateQuery.data?.fee_profile.commission_rate_max ===
    undefined
      ? null
      : defaultMarketTemplateQuery.data.fee_profile.commission_rate_max * 100
  const transactionFeeValidationError =
    commissionRateMaxPercent !== null &&
    effectiveSimulationSettings.transactionFees.commissionRatePercent >
      commissionRateMaxPercent
      ? `佣金率不能高于市场模板上限 ${formatUiPercent(commissionRateMaxPercent)}`
      : null
  const backtestValidateDraft = useMemo<{
    error: string | null
    request: StrategyBacktestValidateRequest | null
  }>(() => {
    if (transactionFeeValidationError) {
      return { error: transactionFeeValidationError, request: null }
    }
    if (!previewSnapshot) {
      return { error: null, request: null }
    }
    if (previewSnapshot.stale) {
      return {
        error: "股池预览已过期，需要先更新股池再执行回测",
        request: null,
      }
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
      return { error: formatErrorMessage(error), request: null }
    }
  }, [
    defaultMarketTemplateQuery.data,
    effectiveSimulationSettings,
    previewSnapshot,
    transactionFeeValidationError,
  ])
  const shouldValidateBacktestDraft =
    activeStep === "simulation" || activeStep === "backtest"
  const backtestDraftQuery = useStrategyBacktestValidateQuery(
    backtestValidateDraft.request,
    shouldValidateBacktestDraft
  )
  const backtestExecutionDraft = useMemo<BacktestExecutionDraft | null>(() => {
    if (!backtestValidateDraft.request || !backtestDraftQuery.data) {
      return null
    }

    return toBacktestExecutionDraft({
      createdAt: backtestDraftQuery.dataUpdatedAt
        ? new Date(backtestDraftQuery.dataUpdatedAt).toISOString()
        : new Date().toISOString(),
      request: backtestValidateDraft.request,
      response: backtestDraftQuery.data,
    })
  }, [
    backtestDraftQuery.data,
    backtestDraftQuery.dataUpdatedAt,
    backtestValidateDraft.request,
  ])
  const backtestValidationError =
    backtestValidateDraft.error ??
    (backtestDraftQuery.isError
      ? formatErrorMessage(backtestDraftQuery.error)
      : null)
  const isBacktestValidationPending = Boolean(
    backtestValidateDraft.request &&
      (backtestDraftQuery.isLoading || backtestDraftQuery.isFetching)
  )
  const canEditConditions = strategyCatalogOptions.length > 0
  const canEditWeights = hasRealScoringCatalog
  const canReuseActiveBacktestRun = Boolean(
    activeBacktestRun &&
      !hasStrategyBacktestConfigChanged(
        activeBacktestRun,
        backtestExecutionDraft,
        backtestPeriod,
        backtestBenchmark
      )
  )
  const canPublishPortfolio = Boolean(
    activeBacktestRun?.status === "succeeded" &&
      activeBacktestRun.current_result_attempt_id &&
      canReuseActiveBacktestRun
  )
  const selectedBacktestPeriodLabel =
    backtestPeriodOptions.find((option) => option.value === backtestPeriod)
      ?.label ?? backtestPeriod
  const selectedBacktestBenchmarkLabel =
    backtestBenchmarkOptions.find(
      (option) => option.securityCode === backtestBenchmark
    )?.label ?? backtestBenchmark
  const publishBacktestRunId = canPublishPortfolio
    ? activeBacktestRun?.strategy_backtest_run_id ?? null
    : null
  const publishNavQuery = useStrategyBacktestNavQuery(
    publishBacktestRunId,
    canPublishPortfolio
  )
  const publishPerformanceQuery = useStrategyBacktestPerformanceQuery(
    publishBacktestRunId,
    canPublishPortfolio
  )
  const publishNavPoints = useMemo(
    () => mapStrategyBacktestNavPoints(publishNavQuery.data ?? []),
    [publishNavQuery.data]
  )
  const publishLatestNetValuePoint = publishNavPoints.at(-1) ?? null
  const publishLatestExcessReturn =
    publishLatestNetValuePoint && publishLatestNetValuePoint.benchmark !== null
      ? formatSignedPercent(
          publishLatestNetValuePoint.strategy -
            publishLatestNetValuePoint.benchmark
        )
      : ""
  const publishPerformanceGroups = useMemo(
    () =>
      buildBacktestPerformanceGroups(
        publishPerformanceQuery.data ?? null,
        publishLatestExcessReturn
      ),
    [publishLatestExcessReturn, publishPerformanceQuery.data]
  )
  const publishConditionRows = useMemo(
    () =>
      conditionGroups.flatMap((group, groupIndex) =>
        group.conditions.map((condition, conditionIndex) => ({
          id: condition.id,
          expression: formatComparableIndicator(condition),
          groupLabel: group.name || `指标组 ${groupIndex + 1}`,
          logicLabel:
            conditionIndex === 0
              ? "组内起始"
              : condition.logic.toUpperCase(),
        }))
      ),
    [conditionGroups]
  )
  const publishScoringRows = useMemo(
    () =>
      weightIndicators.map((indicator, index) => ({
        id: indicator.id,
        expression: formatWeightIndicator(indicator),
        index: index + 1,
        score: indicator.score,
      })),
    [weightIndicators]
  )

  function markRuleDraftChanged() {
    setPreviewAdapterError(null)
    setBacktestLaunchError(null)
    setActiveBacktestRun(null)
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
    setBacktestLaunchError(null)
    setActiveBacktestRun(null)
    setSimulationSettings(nextSettings)
  }

  function handleBack() {
    navigate("/dashboard", { viewTransition: true })
  }

  async function publishPortfolio() {
    if (
      !activeBacktestRun?.current_result_attempt_id ||
      !portfolioName.trim()
    ) {
      return
    }

    await createPortfolioMutation.mutateAsync({
      client_request_id: `strategy-portfolio-${activeBacktestRun.strategy_backtest_run_id}-${activeBacktestRun.current_result_attempt_id}`,
      name: portfolioName.trim(),
      source_result_attempt_id: activeBacktestRun.current_result_attempt_id,
      source_strategy_backtest_run_id: activeBacktestRun.strategy_backtest_run_id,
    })
    await queryClient.invalidateQueries({
      queryKey: queryKeys.strategyPortfolioDashboard(),
    })
    setPublishDialogOpen(false)
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
    const timing = createPreviewTimingLogger()
    setIsOpeningPreview(true)
    previewOpenMutation.reset()
    setPreviewAdapterError(null)
    timing.mark("openPreview:start")

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
      const openedPreview = await previewOpenMutation.mutateAsync({
        end_date: requestRange.end_date,
        preview_row_limit: requestRange.preview_row_limit,
        rule,
        start_date: requestRange.start_date,
      })
      timing.mark("strategy-preview/open:success")
      const latestTradeDate = openedPreview.latest?.trade_date ?? null
      const timeline = {
        end_date: openedPreview.timeline.end_date,
        preview_id: openedPreview.preview_id,
        required_columns: openedPreview.required_columns,
        required_marts: openedPreview.required_marts,
        required_metrics: openedPreview.required_metrics,
        sql_hash: openedPreview.sql_hash,
        start_date: openedPreview.timeline.start_date,
        trade_dates: openedPreview.timeline.trade_dates,
      }
      const result = {
        end_date: latestTradeDate ?? requestRange.end_date,
        preview_id: openedPreview.preview_id,
        preview_row_limit: openedPreview.preview_row_limit,
        required_columns: openedPreview.required_columns,
        required_marts: openedPreview.required_marts,
        required_metrics: openedPreview.required_metrics,
        sql_hash: openedPreview.sql_hash,
        start_date: latestTradeDate ?? requestRange.start_date,
        top_n: openedPreview.top_n,
        trade_dates: openedPreview.latest ? [openedPreview.latest] : [],
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
      setPreviewSnapshot(nextPreviewSnapshot)
      timing.mark("setPreviewSnapshot")
      setActiveStep("preview")
      timing.mark('setActiveStep("preview")')
    } catch (error) {
      timing.mark("openPreview:error")
      setPreviewAdapterError(formatErrorMessage(error))
      if (error instanceof StrategyRuleSpecError) {
        return
      }
    } finally {
      timing.flush()
      setIsOpeningPreview(false)
    }
  }

  async function openBacktest() {
    if (
      !canEnterBacktest ||
      !backtestExecutionDraft ||
      !previewSnapshot
    ) {
      return
    }

    if (canReuseActiveBacktestRun) {
      setBacktestLaunchError(null)
      setActiveStep("backtest")
      return
    }

    setBacktestLaunchError(null)
    setActiveBacktestRun(null)

    try {
      const selectedPeriodOption =
        backtestPeriodOptions.find((option) => option.value === backtestPeriod) ??
        backtestPeriodOptions[0]
      const selectedBenchmarkOption =
        backtestBenchmarkOptions.find(
          (option) => option.securityCode === backtestBenchmark
        ) ?? backtestBenchmarkOptions[0]
      const request = buildStrategyBacktestCreateRequest({
        backtestExecutionDraft,
        benchmark: backtestBenchmark,
        period: backtestPeriod,
        previewSnapshot,
        rangeHint: null,
        selectedBenchmarkLabel: selectedBenchmarkOption.label,
        selectedPeriodDescription: null,
        selectedPeriodLabel: selectedPeriodOption.label,
        settings: effectiveSimulationSettings,
      })
      const run = await initialBacktestMutation.mutateAsync(request)
      const handoff = acceptStrategyBacktestRunForStep5(run)

      setActiveBacktestRun(handoff.activeRun)
      queryClient.setQueryData(
        queryKeys.strategyBacktest(handoff.activeRun.strategy_backtest_run_id),
        handoff.activeRun
      )
      setActiveStep(handoff.activeStep)
    } catch (error) {
      setBacktestLaunchError(formatErrorMessage(error))
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
    if (step === "backtest") {
      void openBacktest()
      return
    }

    setActiveStep(step)
  }

  const content = stepContent[activeStep]
  const isPreviewPending =
    isOpeningPreview ||
    previewOpenMutation.isPending
  const isSplitStep =
    activeStep === "preview" ||
    activeStep === "simulation" ||
    activeStep === "backtest"
  const showStepActions = true
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
      !initialBacktestMutation.isPending
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
                    {previewAdapterError || previewOpenMutation.isError ? (
                      <Alert variant="destructive" className="shrink-0">
                        <AlertTitle>股池预览失败</AlertTitle>
                        <AlertDescription>
                          {previewAdapterError ??
                            formatErrorMessage(previewOpenMutation.error)}
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
                    (previewOpenMutation.isError
                      ? formatErrorMessage(previewOpenMutation.error)
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
                  backtestLaunchError={backtestLaunchError}
                  backtestValidationError={backtestValidationError}
                  catalogOptions={strategyStopLossCatalogOptions}
                  commissionRateMaxPercent={commissionRateMaxPercent}
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
                  activeRun={activeBacktestRun}
                  backtestExecutionDraft={backtestExecutionDraft}
                  backtestValidationError={backtestValidationError}
                  benchmark={backtestBenchmark}
                  isBacktestValidationPending={isBacktestValidationPending}
                  isMarketTemplateError={defaultMarketTemplateQuery.isError}
                  isMarketTemplateLoading={defaultMarketTemplateQuery.isLoading}
                  period={backtestPeriod}
                  previewSnapshot={previewSnapshot}
                  settings={effectiveSimulationSettings}
                  onBenchmarkChange={setBacktestBenchmark}
                  onPeriodChange={setBacktestPeriod}
                  onRunChange={setActiveBacktestRun}
                />
              ) : null}
            </div>

            <Dialog open={publishDialogOpen} onOpenChange={setPublishDialogOpen}>
              <DialogContent className="max-h-[calc(100svh-4rem)] overflow-y-auto sm:max-w-5xl">
                <DialogHeader>
                  <DialogTitle>建立策略组合</DialogTitle>
                  <DialogDescription>
                    确认配置并填写策略名称，发布后返回看板。
                  </DialogDescription>
                </DialogHeader>
                <div className="flex flex-col gap-4 text-xs">
                  <FieldGroup className="grid gap-3 md:grid-cols-[minmax(12rem,1fr)_minmax(18rem,2fr)] md:items-end">
                    <Field>
                      <FieldLabel>策略名称</FieldLabel>
                      <Input
                        value={portfolioName}
                        onChange={(event) => setPortfolioName(event.target.value)}
                        placeholder="请输入策略名称"
                      />
                    </Field>
                    <div className="grid grid-cols-2 gap-2 md:grid-cols-4">
                      <div className="border border-border/70 p-2">
                        <div className="text-muted-foreground">条件指标</div>
                        <div className="mt-1 text-sm font-medium">
                          {publishConditionRows.length} 条
                        </div>
                      </div>
                      <div className="border border-border/70 p-2">
                        <div className="text-muted-foreground">评分项</div>
                        <div className="mt-1 text-sm font-medium">
                          {publishScoringRows.length} 项
                        </div>
                      </div>
                      <div className="border border-border/70 p-2">
                        <div className="text-muted-foreground">候选 / 持仓</div>
                        <div className="mt-1 text-sm font-medium">
                          {effectiveSimulationSettings.buyTopN} /{" "}
                          {effectiveSimulationSettings.maxPositions}
                        </div>
                      </div>
                      <div className="border border-border/70 p-2">
                        <div className="text-muted-foreground">回测结束日</div>
                        <div className="mt-1 text-sm font-medium">
                          {activeBacktestRun?.end_date ?? "—"}
                        </div>
                      </div>
                    </div>
                  </FieldGroup>

                  <div className="grid gap-3 xl:grid-cols-[1.05fr_0.95fr]">
                    <Card>
                      <CardHeader className="pb-2">
                        <CardTitle className="text-sm">条件指标</CardTitle>
                      </CardHeader>
                      <CardContent>
                        <div className="max-h-64 overflow-y-auto border border-border/70">
                          <Table>
                            <TableHeader>
                              <TableRow>
                                <TableHead className="h-8">分组</TableHead>
                                <TableHead className="h-8">关系</TableHead>
                                <TableHead className="h-8">条件表达式</TableHead>
                              </TableRow>
                            </TableHeader>
                            <TableBody>
                              {publishConditionRows.length > 0 ? (
                                publishConditionRows.map((row) => (
                                  <TableRow key={row.id}>
                                    <TableCell className="py-1.5">
                                      {row.groupLabel}
                                    </TableCell>
                                    <TableCell className="py-1.5">
                                      {row.logicLabel}
                                    </TableCell>
                                    <TableCell className="py-1.5 font-mono text-[11px]">
                                      {row.expression}
                                    </TableCell>
                                  </TableRow>
                                ))
                              ) : (
                                <TableRow>
                                  <TableCell
                                    className="py-4 text-center text-muted-foreground"
                                    colSpan={3}
                                  >
                                    暂无条件指标
                                  </TableCell>
                                </TableRow>
                              )}
                            </TableBody>
                          </Table>
                        </div>
                      </CardContent>
                    </Card>

                    <Card>
                      <CardHeader className="pb-2">
                        <CardTitle className="text-sm">评分项</CardTitle>
                      </CardHeader>
                      <CardContent>
                        <div className="max-h-64 overflow-y-auto border border-border/70">
                          <Table>
                            <TableHeader>
                              <TableRow>
                                <TableHead className="h-8">序号</TableHead>
                                <TableHead className="h-8">得分</TableHead>
                                <TableHead className="h-8">评分条件</TableHead>
                              </TableRow>
                            </TableHeader>
                            <TableBody>
                              {publishScoringRows.length > 0 ? (
                                publishScoringRows.map((row) => (
                                  <TableRow key={row.id}>
                                    <TableCell className="py-1.5">
                                      {row.index}
                                    </TableCell>
                                    <TableCell className="py-1.5 font-medium">
                                      +{row.score}
                                    </TableCell>
                                    <TableCell className="py-1.5 font-mono text-[11px]">
                                      {row.expression}
                                    </TableCell>
                                  </TableRow>
                                ))
                              ) : (
                                <TableRow>
                                  <TableCell
                                    className="py-4 text-center text-muted-foreground"
                                    colSpan={3}
                                  >
                                    暂无评分项
                                  </TableCell>
                                </TableRow>
                              )}
                            </TableBody>
                          </Table>
                        </div>
                      </CardContent>
                    </Card>
                  </div>

                  <div className="grid gap-3 xl:grid-cols-[0.95fr_1.05fr]">
                    <Card>
                      <CardHeader className="pb-2">
                        <CardTitle className="text-sm">建仓摘要</CardTitle>
                      </CardHeader>
                      <CardContent className="grid grid-cols-2 gap-2 md:grid-cols-3">
                        <div>
                          <div className="text-muted-foreground">初始资金</div>
                          <div className="mt-1 font-medium">
                            {formatCurrency(
                              effectiveSimulationSettings.initialCapital
                            )}
                          </div>
                        </div>
                        <div>
                          <div className="text-muted-foreground">每日候选</div>
                          <div className="mt-1 font-medium">
                            Top {effectiveSimulationSettings.buyTopN}
                          </div>
                        </div>
                        <div>
                          <div className="text-muted-foreground">最大持仓</div>
                          <div className="mt-1 font-medium">
                            {effectiveSimulationSettings.maxPositions} 只
                          </div>
                        </div>
                        <div>
                          <div className="text-muted-foreground">单票上限</div>
                          <div className="mt-1 font-medium">
                            {formatUiPercent(
                              effectiveSimulationSettings.singlePositionLimitPercent
                            )}
                          </div>
                        </div>
                        <div>
                          <div className="text-muted-foreground">佣金率</div>
                          <div className="mt-1 font-medium">
                            {formatUiPercent(
                              effectiveSimulationSettings.transactionFees
                                .commissionRatePercent
                            )}
                          </div>
                        </div>
                        <div>
                          <div className="text-muted-foreground">滑点</div>
                          <div className="mt-1 font-medium">
                            {formatUiPercent(
                              effectiveSimulationSettings.transactionFees
                                .slippageRatePercent
                            )}
                          </div>
                        </div>
                        <div>
                          <div className="text-muted-foreground">印花税</div>
                          <div className="mt-1 font-medium">
                            {formatUiPercent(
                              effectiveSimulationSettings.transactionFees
                                .stampDutyRatePercent
                            )}
                          </div>
                        </div>
                        <div>
                          <div className="text-muted-foreground">过户费</div>
                          <div className="mt-1 font-medium">
                            {formatUiPercent(
                              effectiveSimulationSettings.transactionFees
                                .transferFeeRatePercent
                            )}
                          </div>
                        </div>
                        <div>
                          <div className="text-muted-foreground">风控规则</div>
                          <div className="mt-1 font-medium">
                            {backtestExecutionDraft?.summary
                              .enabled_exit_rule_count ?? 0}{" "}
                            条启用
                          </div>
                        </div>
                        <div className="col-span-2 md:col-span-3">
                          <div className="text-muted-foreground">风控摘要</div>
                          <div className="mt-1 leading-5">
                            固定止损{" "}
                            {effectiveSimulationSettings.fixedStopLoss.enabled
                              ? formatUiPercent(
                                  effectiveSimulationSettings.fixedStopLoss
                                    .lossPercent
                                )
                              : "未启用"}
                            ，止盈{" "}
                            {effectiveSimulationSettings.takeProfit.enabled
                              ? formatUiPercent(
                                  effectiveSimulationSettings.takeProfit
                                    .profitPercent
                                )
                              : "未启用"}
                            ，时间止损{" "}
                            {effectiveSimulationSettings.timeStopLoss.enabled
                              ? `${effectiveSimulationSettings.timeStopLoss.holdingDays} 天`
                              : "未启用"}
                            ，指标止损{" "}
                            {effectiveSimulationSettings.indicatorStopLoss.enabled
                              ? effectiveSimulationSettings.indicatorStopLoss.metric
                              : "未启用"}
                          </div>
                        </div>
                      </CardContent>
                    </Card>

                    <Card>
                      <CardHeader className="pb-2">
                        <CardTitle className="text-sm">回测业绩</CardTitle>
                      </CardHeader>
                      <CardContent className="flex flex-col gap-3">
                        {publishNavQuery.isLoading ||
                        publishPerformanceQuery.isLoading ? (
                          <Skeleton className="h-28 w-full" />
                        ) : (
                          <>
                            <div className="grid grid-cols-2 gap-2 md:grid-cols-4">
                              <div>
                                <div className="text-muted-foreground">
                                  最新交易日
                                </div>
                                <div className="mt-1 font-medium">
                                  {publishLatestNetValuePoint?.time ?? "—"}
                                </div>
                              </div>
                              <div>
                                <div className="text-muted-foreground">
                                  策略净值
                                </div>
                                <div className="mt-1 font-medium">
                                  {publishLatestNetValuePoint
                                    ? formatNetValue(
                                        publishLatestNetValuePoint.strategy
                                      )
                                    : "—"}
                                </div>
                              </div>
                              <div>
                                <div className="text-muted-foreground">
                                  基准净值
                                </div>
                                <div className="mt-1 font-medium">
                                  {publishLatestNetValuePoint?.benchmark !== null &&
                                  publishLatestNetValuePoint?.benchmark !==
                                    undefined
                                    ? formatNetValue(
                                        publishLatestNetValuePoint.benchmark
                                      )
                                    : "—"}
                                </div>
                              </div>
                              <div>
                                <div className="text-muted-foreground">
                                  日胜率
                                </div>
                                <div className="mt-1 font-medium">
                                  {formatOptionalPercent(
                                    publishPerformanceQuery.data?.daily_win_rate
                                      .value
                                  )}
                                </div>
                              </div>
                            </div>
                            <div className="grid gap-2 md:grid-cols-2">
                              {publishPerformanceGroups.map((group) => (
                                <div
                                  className="border border-border/70 p-2"
                                  key={group.title}
                                >
                                  <div className="mb-1 font-medium">
                                    {group.title}
                                  </div>
                                  <div className="grid grid-cols-2 gap-x-3 gap-y-1">
                                    {group.metrics.map((metric) => (
                                      <Fragment
                                        key={`${group.title}-${metric.label}`}
                                      >
                                        <div className="text-muted-foreground">
                                          {metric.label}
                                        </div>
                                        <div className="text-right font-medium">
                                          {metric.value}
                                        </div>
                                      </Fragment>
                                    ))}
                                  </div>
                                </div>
                              ))}
                            </div>
                            <div className="text-muted-foreground">
                              日胜率样本{" "}
                              {publishPerformanceQuery.data?.daily_win_rate
                                .winning_day_count ?? "—"}{" "}
                              /{" "}
                              {publishPerformanceQuery.data?.daily_win_rate
                                .observation_count ?? "—"}
                            </div>
                          </>
                        )}
                        {publishNavQuery.isError ||
                        publishPerformanceQuery.isError ? (
                          <Alert variant="destructive">
                            <AlertTitle>回测业绩读取失败</AlertTitle>
                            <AlertDescription>
                              {publishNavQuery.isError
                                ? formatErrorMessage(publishNavQuery.error)
                                : formatErrorMessage(
                                    publishPerformanceQuery.error
                                  )}
                            </AlertDescription>
                          </Alert>
                        ) : null}
                      </CardContent>
                    </Card>
                  </div>

                  <Card>
                    <CardHeader className="pb-2">
                      <CardTitle className="text-sm">回测快照</CardTitle>
                    </CardHeader>
                    <CardContent className="grid gap-2 md:grid-cols-3">
                      <div>
                        <div className="text-muted-foreground">周期 / 基准</div>
                        <div className="mt-1 font-medium">
                          {selectedBacktestPeriodLabel} /{" "}
                          {selectedBacktestBenchmarkLabel}
                        </div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">回测区间</div>
                        <div className="mt-1 font-medium">
                          {activeBacktestRun
                            ? `${activeBacktestRun.start_date} - ${activeBacktestRun.end_date}`
                            : "—"}
                        </div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">当前结果</div>
                        <div className="mt-1 font-medium">
                          {activeBacktestRun?.current_result_attempt_id ?? "—"}
                        </div>
                      </div>
                      <div className="md:col-span-2">
                        <div className="text-muted-foreground">回测 Run ID</div>
                        <div className="mt-1 break-all font-mono text-[11px]">
                          {activeBacktestRun?.strategy_backtest_run_id ?? "—"}
                        </div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">股池预览</div>
                        <div className="mt-1 font-medium">
                          {previewSnapshot?.range.selectedTradeDate ??
                            previewSnapshot?.range.endDate ??
                            "—"}
                        </div>
                      </div>
                    </CardContent>
                  </Card>

                  {createPortfolioMutation.isError ? (
                    <Alert variant="destructive">
                      <AlertTitle>组合创建失败</AlertTitle>
                      <AlertDescription>
                        {formatErrorMessage(createPortfolioMutation.error)}
                      </AlertDescription>
                    </Alert>
                  ) : null}

                  <div className="flex justify-end gap-2">
                    <Button
                      variant="outline"
                      type="button"
                      onClick={() => setPublishDialogOpen(false)}
                    >
                      取消
                    </Button>
                    <Button
                      disabled={
                        !portfolioName.trim() ||
                        !canPublishPortfolio ||
                        createPortfolioMutation.isPending
                      }
                      type="button"
                      onClick={() => void publishPortfolio()}
                    >
                      {createPortfolioMutation.isPending ? (
                        <Spinner data-icon="inline-start" />
                      ) : null}
                      确定
                    </Button>
                  </div>
                </div>
              </DialogContent>
            </Dialog>

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
                          onClick={() => void openBacktest()}
                          type="button"
                        >
                          {isBacktestValidationPending ||
                          initialBacktestMutation.isPending ? (
                            <Spinner data-icon="inline-start" />
                          ) : (
                            <Play data-icon="inline-start" />
                          )}
                          策略回测
                        </Button>
                      </div>
                      <div className="hidden xl:block" />
                      <div className="hidden xl:block" />
                    </>
                  ) : activeStep === "backtest" ? (
                    <>
                      <div className="flex flex-wrap items-center gap-2">
                        <Button
                          variant="default"
                          size="lg"
                          className="w-full sm:w-48"
                          disabled={!canPublishPortfolio}
                          onClick={() => {
                            setPortfolioName("策略组合")
                            setPublishDialogOpen(true)
                          }}
                          type="button"
                        >
                          建立组合
                        </Button>
                      </div>
                      <div className="hidden xl:block" />
                      <div className="hidden xl:block" />
                    </>
                  ) : null}
                </div>
              </>
            ) : null}
          </div>
        </main>
      </div>
    </section>
  )
}
