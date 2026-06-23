import { Fragment, useEffect, useMemo, useRef, useState } from "react"
import { useNavigate } from "react-router-dom"
import { createChart, LineSeries } from "lightweight-charts"
import { useQueryClient } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import { securityAnalysis } from "@/api/rearview"
import {
  useDefaultMarketFeeTemplateQuery,
  useMetricsQuery,
  useStrategyBacktestCreateMutation,
  useStrategyBacktestNavQuery,
  useStrategyBacktestOptionsQuery,
  useStrategyBacktestPerformanceQuery,
  useStrategyBacktestQuery,
  useStrategyBacktestRebalanceRecordsQuery,
  useStrategyBacktestValidateQuery,
  useStrategyPreviewMutation,
  useStrategyPreviewTimelineMutation,
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
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
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
import type {
  StrategyBacktestNavPoint,
  StrategyBacktestPerformanceView,
  StrategyBacktestRebalanceRecord as ApiBacktestRebalanceRecord,
  StrategyBacktestRunRecord,
  StrategyBacktestRunStatus,
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
const previewAnalysisMaWindows = "5,10,30"

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
  backtestExecutionDraft,
  backtestValidationError,
  benchmark,
  isBacktestValidationPending,
  isMarketTemplateError,
  isMarketTemplateLoading,
  onBenchmarkChange,
  onPeriodChange,
  period,
  previewSnapshot,
  settings,
}: {
  backtestExecutionDraft: BacktestExecutionDraft | null
  backtestValidationError: string | null
  benchmark: BacktestBenchmark
  isBacktestValidationPending: boolean
  isMarketTemplateError: boolean
  isMarketTemplateLoading: boolean
  onBenchmarkChange: (benchmark: BacktestBenchmark) => void
  onPeriodChange: (period: BacktestPeriod) => void
  period: BacktestPeriod
  previewSnapshot: PreviewSnapshot | null
  settings: SimulationSettings
}) {
  const [activeRunId, setActiveRunId] = useState<string | null>(null)
  const [selectedRebalanceDate, setSelectedRebalanceDate] = useState<
    string | null
  >(null)
  const optionsQuery = useStrategyBacktestOptionsQuery(benchmark)
  const createBacktestMutation = useStrategyBacktestCreateMutation()
  const runQuery = useStrategyBacktestQuery(activeRunId)
  const currentRun =
    runQuery.data ??
    (createBacktestMutation.data?.strategy_backtest_run_id === activeRunId
      ? createBacktestMutation.data
      : null)
  const hasPendingConfigChange = hasStrategyBacktestConfigChanged(
    currentRun,
    backtestExecutionDraft,
    period,
    benchmark
  )
  const isRunInProgress = Boolean(
    currentRun && !isStrategyBacktestTerminalStatus(currentRun.status)
  )
  const isResultReady = Boolean(
    currentRun?.status === "succeeded" &&
      currentRun.current_result_attempt_id &&
      !hasPendingConfigChange
  )
  const navQuery = useStrategyBacktestNavQuery(activeRunId, isResultReady)
  const rebalanceRecordsQuery = useStrategyBacktestRebalanceRecordsQuery(
    activeRunId,
    selectedRebalanceDate,
    isResultReady
  )
  const performanceQuery = useStrategyBacktestPerformanceQuery(
    activeRunId,
    isResultReady
  )

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
    () => mapStrategyBacktestNavPoints(navQuery.data ?? []),
    [navQuery.data]
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
      (rebalanceRecordsQuery.data?.records ?? []).map(
        mapApiBacktestRebalanceRecord
      ),
    [rebalanceRecordsQuery.data]
  )
  const selectedRebalanceRecord =
    rebalanceRecords.find(
      (record) => record.date === rebalanceRecordsQuery.data?.selected_trade_date
    ) ??
    rebalanceRecords.find((record) => record.date === selectedRebalanceDate) ??
    rebalanceRecords.at(-1)
  const selectedRebalanceTradeSections = selectedRebalanceRecord
    ? buildRebalanceTradeSections(selectedRebalanceRecord.trades)
    : []
  const performanceGroups = useMemo(
    () =>
      buildBacktestPerformanceGroups(
        performanceQuery.data ?? null,
        latestExcessReturn
      ),
    [latestExcessReturn, performanceQuery.data]
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
      createBacktestMutation.isPending ||
      isRunInProgress
  )
  const actionLabel = createBacktestMutation.isPending
    ? "提交中"
    : isRunInProgress
      ? getStrategyBacktestStatusLabel(currentRun?.status)
      : activeRunId
        ? "重新回测"
        : "开始回测"
  const showActionSpinner =
    createBacktestMutation.isPending ||
    isRunInProgress ||
    isBacktestValidationPending ||
    optionsQuery.isLoading

  async function runBacktest() {
    if (!backtestExecutionDraft || !previewSnapshot) {
      return
    }

    const request = {
      ...buildBacktestExecutionRequestDraft({
        benchmark,
        draft: backtestExecutionDraft,
        period,
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
          resolved_range: selectedPeriodOption?.description ?? null,
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
    const run = await createBacktestMutation.mutateAsync(request)
    setActiveRunId(run.strategy_backtest_run_id)
    setSelectedRebalanceDate(null)
  }

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
              {selectedPeriodOption?.description ? (
                <FieldDescription>
                  {selectedPeriodOption.description}
                </FieldDescription>
              ) : null}
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
                        disabled={option.availabilityStatus !== "available"}
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
            hasPendingConfigChange={hasPendingConfigChange}
            isMarketTemplateError={isMarketTemplateError}
            optionsError={optionsQuery.error}
            run={currentRun}
            runError={runQuery.error}
          />

          <Separator className="bg-border/60" />

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
            {navQuery.isLoading ? (
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

            {rebalanceRecordsQuery.isLoading ? (
              <Skeleton className="h-36 w-full" />
            ) : rebalanceRecords.length > 0 ? (
              <>
                <div className="h-[32px] shrink-0 overflow-x-auto overflow-y-hidden overscroll-x-contain pb-3 [scrollbar-width:thin] [&::-webkit-scrollbar]:h-[2px] [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent">
                  <div className="flex min-w-max gap-1.5 pr-1">
                    {rebalanceRecords.map((record) => {
                  const isSelected =
                    record.date === selectedRebalanceRecord?.date

                  return (
                    <Button
                      key={record.date}
                      aria-label={`${record.date} 持仓 ${record.positionCount} 只`}
                      aria-pressed={isSelected}
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
                    {selectedRebalanceTradeSections.some(
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
  hasPendingConfigChange,
  isMarketTemplateError,
  optionsError,
  run,
  runError,
}: {
  backtestValidationError: string | null
  createError: unknown
  hasPendingConfigChange: boolean
  isMarketTemplateError: boolean
  optionsError: unknown
  run: StrategyBacktestRunRecord | null
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

  if (run && !isStrategyBacktestTerminalStatus(run.status)) {
    return (
      <Alert>
        <AlertTitle>{getStrategyBacktestStatusLabel(run.status)}</AlertTitle>
        <AlertDescription>
          回测任务已进入异步队列，页面会自动刷新状态。
        </AlertDescription>
      </Alert>
    )
  }

  if (hasPendingConfigChange) {
    return (
      <Alert>
        <AlertTitle>配置已变更</AlertTitle>
        <AlertDescription>
          当前展示结果不再匹配所选周期、基准或策略配置，需要重新回测。
        </AlertDescription>
      </Alert>
    )
  }

  return null
}

function isStrategyBacktestTerminalStatus(
  status: StrategyBacktestRunStatus
): boolean {
  return (
    status === "succeeded" ||
    status === "cancelled" ||
    isStrategyBacktestFailedStatus(status)
  )
}

function isStrategyBacktestFailedStatus(status: StrategyBacktestRunStatus) {
  return status.startsWith("failed_")
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

function hasStrategyBacktestConfigChanged(
  run: StrategyBacktestRunRecord | null,
  draft: BacktestExecutionDraft | null,
  period: BacktestPeriod,
  benchmark: BacktestBenchmark
) {
  if (!run || !draft) {
    return false
  }

  return (
    run.period_key !== period ||
    run.benchmark_security_code !== benchmark ||
    run.rule_hash !== draft.rule_hash ||
    run.execution_config_hash !== draft.execution_config_hash
  )
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

function mapApiBacktestRebalanceRecord(
  record: ApiBacktestRebalanceRecord
): BacktestRebalanceRecord {
  return {
    buyCount: record.buy_count,
    date: record.trade_date,
    holdCount: record.hold_count,
    positionCount: record.position_count,
    sellCount: record.sell_count,
    trades: record.rows.map((row) => ({
      changePercent: formatOptionalSignedPercent(row.change_pct),
      contribution: formatOptionalSignedPercent(row.contribution_pct),
      costPrice: formatOptionalCurrency(row.cost_price),
      currentPrice: formatOptionalCurrency(row.current_price),
      direction: row.direction,
      holdingDays:
        typeof row.holding_days === "number" ? `${row.holding_days}天` : "—",
      securityCode: row.security_code,
      securityName: row.security_name?.trim() || row.security_code,
    })),
  }
}

function buildBacktestPerformanceGroups(
  performance: StrategyBacktestPerformanceView | null,
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
  performance: StrategyBacktestPerformanceView | null,
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
  const backtestDraftQuery = useStrategyBacktestValidateQuery(
    backtestValidateDraft.request
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

  function openBacktest() {
    if (!canEnterBacktest) {
      return
    }

    setActiveStep("backtest")
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
      openBacktest()
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
      !isBacktestValidationPending
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
                          onClick={openBacktest}
                          type="button"
                        >
                          {isBacktestValidationPending ? (
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
                  ) : null}
                </div>
              </>
            ) : null}

            {activeStep === "backtest" ? <Separator /> : null}
          </div>
        </main>
      </div>
    </section>
  )
}
