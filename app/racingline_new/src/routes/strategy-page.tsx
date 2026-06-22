import { Fragment, useEffect, useMemo, useRef, useState } from "react"
import { useNavigate } from "react-router-dom"
import { createChart, LineSeries } from "lightweight-charts"

import {
  useExplainMutation,
  useMetricsQuery,
  useStrategyPreviewMutation,
} from "@/api/hooks"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
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
  buildStrategySelectionRuleSpec,
  StrategyRuleSpecError,
} from "@/features/strategy/adapters"
import { indicatorCatalog } from "@/features/strategy/catalog"
import { ConditionGroupsPanel } from "@/features/strategy/components/condition-groups-panel"
import { PoolPreviewPanel } from "@/features/strategy/components/pool-preview-panel"
import type { PreviewRange } from "@/features/strategy/components/pool-preview-panel"
import { SimulationPositionPanel } from "@/features/strategy/components/simulation-position-panel"
import { StrategyStepSidebar } from "@/features/strategy/components/strategy-step-sidebar"
import { WeightIndicatorsPanel } from "@/features/strategy/components/weight-indicators-panel"
import type {
  SimulationSettings,
  Step,
  StrategyCondition,
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import {
  clampScore,
  createCondition,
  createId,
  createWeightIndicator,
} from "@/features/strategy/utils"
import { cn } from "@/lib/utils"
import type {
  ExplainResponse,
  RuleVersionSpec,
  StrategyPreviewResponse,
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
  buyTopN: 10,
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

const defaultPreviewRange: PreviewRange = {
  startDate: "2026-05-26",
  endDate: "2026-06-01",
  topN: 10,
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

const backtestPerformanceGroups = [
  {
    title: "收益指标",
    metrics: [
      { label: "持仓收益", value: "+18.42%" },
      { label: "年化收益", value: "+21.96%" },
      { label: "日胜率", value: "56.73%" },
    ],
  },
  {
    title: "风险指标",
    metrics: [
      { label: "最大回撤", value: "-8.24%" },
      { label: "年化波动率", value: "13.75%" },
      { label: "下行波动率", value: "9.41%" },
    ],
  },
  {
    title: "性价比",
    metrics: [
      { label: "Sharpe Ratio", value: "1.42" },
      { label: "Sortino Ratio", value: "1.91" },
      { label: "Calmar Ratio", value: "2.66" },
      { label: "Treynor Ratio", value: "0.23" },
    ],
  },
  {
    title: "相对市场",
    metrics: [
      { label: "Alpha", value: "4.10%" },
      { label: "Beta", value: "0.78" },
      { label: "Information Ratio", value: "0.88" },
    ],
  },
] as const

const backtestNetValuePoints = [
  { time: "2025-06-20", strategy: 1.0, benchmark: 1.0 },
  { time: "2025-07-04", strategy: 1.014, benchmark: 1.006 },
  { time: "2025-07-18", strategy: 1.031, benchmark: 1.012 },
  { time: "2025-08-01", strategy: 1.049, benchmark: 1.017 },
  { time: "2025-08-15", strategy: 1.083, benchmark: 1.029 },
  { time: "2025-08-29", strategy: 1.102, benchmark: 1.038 },
  { time: "2025-09-12", strategy: 1.126, benchmark: 1.051 },
  { time: "2025-09-26", strategy: 1.107, benchmark: 1.043 },
  { time: "2025-10-10", strategy: 1.139, benchmark: 1.058 },
  { time: "2025-10-24", strategy: 1.157, benchmark: 1.066 },
  { time: "2025-11-07", strategy: 1.174, benchmark: 1.073 },
  { time: "2025-11-21", strategy: 1.162, benchmark: 1.069 },
  { time: "2025-12-05", strategy: 1.1842, benchmark: 1.082 },
] as const

const backtestTradeCandidates = [
  { securityCode: "600519.SH", securityName: "贵州茅台", basePrice: 1458.2 },
  { securityCode: "300750.SZ", securityName: "宁德时代", basePrice: 187.64 },
  { securityCode: "601318.SH", securityName: "中国平安", basePrice: 45.12 },
  { securityCode: "000858.SZ", securityName: "五粮液", basePrice: 132.4 },
  { securityCode: "600036.SH", securityName: "招商银行", basePrice: 34.15 },
  { securityCode: "600276.SH", securityName: "恒瑞医药", basePrice: 48.72 },
  { securityCode: "002415.SZ", securityName: "海康威视", basePrice: 31.06 },
  { securityCode: "002594.SZ", securityName: "比亚迪", basePrice: 236.5 },
  { securityCode: "600900.SH", securityName: "长江电力", basePrice: 28.36 },
  { securityCode: "688981.SH", securityName: "中芯国际", basePrice: 57.42 },
] as const

const backtestRebalanceRecords = buildBacktestRebalanceRecords()

const splitStepLayoutClassName = "xl:grid-cols-[minmax(34rem,1fr)_auto_20rem]"

type BacktestPeriod = (typeof backtestPeriodOptions)[number]["value"]
type BacktestBenchmark =
  (typeof backtestBenchmarkOptions)[number]["securityCode"]
type BacktestNetValuePoint = (typeof backtestNetValuePoints)[number]
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
  date: string
  trades: BacktestRebalanceTrade[]
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
    top_n: Math.max(1, Math.floor(previewRange.topN || 1)),
  }
}

function BacktestPanel({
  benchmark,
  onBenchmarkChange,
  onPeriodChange,
  period,
}: {
  benchmark: BacktestBenchmark
  onBenchmarkChange: (benchmark: BacktestBenchmark) => void
  onPeriodChange: (period: BacktestPeriod) => void
  period: BacktestPeriod
}) {
  const [selectedRebalanceDate, setSelectedRebalanceDate] = useState(
    backtestRebalanceRecords.at(-1)?.date ?? ""
  )
  const selectedPeriodLabel =
    backtestPeriodOptions.find((option) => option.value === period)?.label ??
    period
  const selectedBenchmarkLabel =
    backtestBenchmarkOptions.find((option) => option.securityCode === benchmark)
      ?.label ?? benchmark
  const selectedRebalanceRecord =
    backtestRebalanceRecords.find(
      (record) => record.date === selectedRebalanceDate
    ) ?? backtestRebalanceRecords.at(-1)
  const selectedRebalanceTradeSections = selectedRebalanceRecord
    ? buildRebalanceTradeSections(selectedRebalanceRecord.trades)
    : []
  const latestNetValuePoint = backtestNetValuePoints.at(-1)
  const latestExcessReturn = latestNetValuePoint
    ? formatSignedPercent(
        latestNetValuePoint.strategy - latestNetValuePoint.benchmark
      )
    : ""

  return (
    <div className="grid min-h-full gap-y-4 xl:grid-cols-[minmax(34rem,1fr)_auto_20rem] xl:gap-x-0">
      <div className="flex min-h-0 flex-col gap-4 pt-7">
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
              variant="outline"
              size="lg"
              type="button"
            >
              重新回测
            </Button>
          </FieldGroup>

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
            <BacktestNetValueChart points={backtestNetValuePoints} />
          </section>

          <Separator className="bg-border/60" />

          <section className="flex flex-col gap-3 xl:pr-4">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-medium">持仓记录</div>
              <div className="text-xs text-muted-foreground tabular-nums">
                {backtestRebalanceRecords.length} 个调仓日
              </div>
            </div>

            <div className="h-[32px] shrink-0 [scrollbar-width:thin] overflow-x-auto overflow-y-hidden overscroll-x-contain pb-3 [&::-webkit-scrollbar]:h-[2px] [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent">
              <div className="flex min-w-max gap-1.5 pr-1">
                {backtestRebalanceRecords.map((record) => {
                  const isSelected =
                    record.date === selectedRebalanceRecord?.date
                  const buyCount = record.trades.filter(
                    (trade) => trade.direction === "buy"
                  ).length
                  const holdCount = record.trades.filter(
                    (trade) => trade.direction === "hold"
                  ).length
                  const positionCount = buyCount + holdCount

                  return (
                    <Button
                      key={record.date}
                      aria-label={`${record.date} 持仓 ${positionCount} 只`}
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
                        {positionCount}只
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
                    调入{" "}
                    {
                      selectedRebalanceRecord.trades.filter(
                        (trade) => trade.direction === "buy"
                      ).length
                    }{" "}
                    只 / 持有{" "}
                    {
                      selectedRebalanceRecord.trades.filter(
                        (trade) => trade.direction === "hold"
                      ).length
                    }{" "}
                    只 / 卖出{" "}
                    {
                      selectedRebalanceRecord.trades.filter(
                        (trade) => trade.direction === "sell"
                      ).length
                    }{" "}
                    只
                  </div>
                </div>
                <Table className="w-full table-fixed text-xs leading-snug [&_td]:overflow-hidden [&_th]:overflow-hidden">
                  <TableHeader>
                    <TableRow className="hover:bg-transparent">
                      <TableHead className="h-6 w-[20%] px-1">股票</TableHead>
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
                                getSignedValueClassName(trade.changePercent)
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
                                getSignedValueClassName(trade.contribution)
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
              </div>
            ) : null}
          </section>
        </div>
      </div>

      <Separator className="xl:hidden" />
      <Separator className="hidden xl:block" orientation="vertical" />

      <div className="flex min-h-0 flex-col gap-4 pt-7">
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
                <div className="grid grid-cols-2 gap-2">
                  <BacktestSummaryMetric
                    label="策略净值"
                    value={formatNetValue(latestNetValuePoint.strategy)}
                  />
                  <BacktestSummaryMetric
                    label="基准净值"
                    value={formatNetValue(latestNetValuePoint.benchmark)}
                  />
                </div>

                <Separator />
              </>
            ) : null}

            <div className="flex flex-col gap-4">
              {backtestPerformanceGroups.map((group) => {
                const metrics =
                  group.title === "收益指标" && latestExcessReturn
                    ? [
                        ...group.metrics.slice(0, 2),
                        { label: "超额收益", value: latestExcessReturn },
                        ...group.metrics.slice(2),
                      ]
                    : group.metrics

                return (
                  <div
                    key={group.title}
                    className="flex min-w-0 flex-col gap-2"
                  >
                    <div className="text-xs font-medium text-muted-foreground">
                      {group.title}
                    </div>
                    <div className="flex flex-col gap-1.5">
                      {metrics.map((metric) => (
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
                )
              })}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
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
  if (label === "持仓收益" || label === "年化收益") {
    return "text-[color:var(--portfolio-up)]"
  }

  if (label === "最大回撤") {
    return "text-[color:var(--portfolio-down)]"
  }

  if (label === "超额收益") {
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

function buildBacktestRebalanceRecords(): BacktestRebalanceRecord[] {
  return buildTradingDates("2025-01-02", 252).map((date, dayIndex) => {
    const buyCount = 1 + (dayIndex % 3 === 0 ? 1 : 0)
    const holdCount = 2 + (dayIndex % 4)
    const sellCount = dayIndex < 8 ? 1 : 1 + (dayIndex % 4 === 0 ? 1 : 0)
    const buys = Array.from({ length: buyCount }, (_, index) =>
      buildBacktestTrade("buy", dayIndex, index)
    )
    const holds = Array.from({ length: holdCount }, (_, index) =>
      buildBacktestTrade("hold", dayIndex, index + buyCount)
    )
    const sells = Array.from({ length: sellCount }, (_, index) =>
      buildBacktestTrade("sell", dayIndex, index + buyCount + holdCount)
    )

    return {
      date,
      trades: [...buys, ...holds, ...sells],
    }
  })
}

function buildBacktestTrade(
  direction: BacktestTradeDirection,
  dayIndex: number,
  offset: number
): BacktestRebalanceTrade {
  const candidate =
    backtestTradeCandidates[
      (dayIndex * 3 + offset * 5) % backtestTradeCandidates.length
    ]
  const costPrice =
    candidate.basePrice * (0.96 + ((dayIndex + offset) % 9) / 50)
  const change =
    direction === "buy"
      ? 0
      : Math.sin((dayIndex + offset) * 0.71) * 0.075 + (dayIndex % 5) * 0.006
  const currentPrice = costPrice * (1 + change)
  const contribution =
    direction === "buy" ? 0 : change * (0.08 + ((dayIndex + offset) % 4) * 0.02)

  return {
    changePercent: formatSignedPercent(change),
    contribution: formatSignedPercent(contribution),
    costPrice: formatCurrency(costPrice),
    currentPrice: formatCurrency(currentPrice),
    direction,
    holdingDays:
      direction === "buy"
        ? "0天"
        : `${8 + ((dayIndex * 7 + offset * 3) % 96)}天`,
    securityCode: candidate.securityCode,
    securityName: candidate.securityName,
  }
}

function buildTradingDates(startDate: string, count: number) {
  const dates: string[] = []
  const date = new Date(`${startDate}T00:00:00Z`)

  while (dates.length < count) {
    const day = date.getUTCDay()

    if (day !== 0 && day !== 6) {
      dates.push(formatIsoDate(date))
    }

    date.setUTCDate(date.getUTCDate() + 1)
  }

  return dates
}

function formatIsoDate(date: Date) {
  const year = date.getUTCFullYear()
  const month = String(date.getUTCMonth() + 1).padStart(2, "0")
  const day = String(date.getUTCDate()).padStart(2, "0")

  return `${year}-${month}-${day}`
}

function formatCompactDate(date: string) {
  const [, month, day] = date.split("-")

  return `${month}/${day}`
}

function formatCurrency(value: number) {
  return `¥${value.toFixed(2)}`
}

function formatNetValue(value: number) {
  return value.toFixed(4)
}

function formatSignedPercent(value: number) {
  const sign = value > 0 ? "+" : ""

  return `${sign}${(value * 100).toFixed(2)}%`
}

function formatErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message
  }

  return String(error || "Unknown error")
}

function formatRequiredColumns(columns: Record<string, string[]> | undefined) {
  if (!columns) {
    return ""
  }

  return Object.entries(columns)
    .map(([mart, names]) => `${mart}: ${names.join(", ")}`)
    .join(" / ")
}

function BacktestNetValueChart({
  points,
}: {
  points: readonly BacktestNetValuePoint[]
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const container = containerRef.current

    if (!container) {
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
      points.map((point) => ({
        time: point.time,
        value: point.benchmark,
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
  usingFallback,
}: {
  error: unknown
  isError: boolean
  isLoading: boolean
  usingFallback: boolean
}) {
  if (isError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>指标加载失败</AlertTitle>
        <AlertDescription>
          {usingFallback
            ? `${formatErrorMessage(error)} / 当前仅展示原型字段，真实校验已禁用。`
            : formatErrorMessage(error)}
        </AlertDescription>
      </Alert>
    )
  }

  if (isLoading && usingFallback) {
    return (
      <Alert>
        <Spinner />
        <AlertTitle>正在加载真实指标目录</AlertTitle>
        <AlertDescription>
          当前先展示原型字段；Rearview metrics 返回前，真实校验保持禁用。
        </AlertDescription>
      </Alert>
    )
  }

  if (isLoading) {
    return <Skeleton className="h-9 w-full" />
  }

  if (usingFallback) {
    return (
      <Alert>
        <AlertTitle>使用原型字段</AlertTitle>
        <AlertDescription>
          Rearview catalog
          当前没有可用指标；页面保留编辑器展示，真实校验已禁用。
        </AlertDescription>
      </Alert>
    )
  }

  return null
}

function ExplainStatusPanel({
  adapterError,
  error,
  isPending,
  lastExplainAt,
  result,
  ruleSpec,
  stale,
}: {
  adapterError: string | null
  error: unknown
  isPending: boolean
  lastExplainAt: string | null
  result: ExplainResponse | null
  ruleSpec: RuleVersionSpec | null
  stale: boolean
}) {
  if (adapterError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>规则草案无效</AlertTitle>
        <AlertDescription>{adapterError}</AlertDescription>
      </Alert>
    )
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertTitle>规则校验失败</AlertTitle>
        <AlertDescription>{formatErrorMessage(error)}</AlertDescription>
      </Alert>
    )
  }

  if (isPending) {
    return (
      <Alert>
        <Spinner />
        <AlertTitle>正在校验规则</AlertTitle>
        <AlertDescription>Rearview explain 正在编译规则草案。</AlertDescription>
      </Alert>
    )
  }

  if (!result || !ruleSpec) {
    return null
  }

  return (
    <div className="flex flex-col gap-3">
      <Alert>
        <AlertTitle>{stale ? "规则校验已过期" : "规则校验通过"}</AlertTitle>
        <AlertDescription>
          {[
            result.sql_hash ?? result.compiled_sql_hash ?? "no-hash",
            `${result.required_metrics?.length ?? 0} metrics`,
            `${result.required_marts?.length ?? 0} marts`,
            `${result.chunk_plan?.length ?? 0} chunks`,
            lastExplainAt ?? "",
          ]
            .filter(Boolean)
            .join(" / ")}
        </AlertDescription>
      </Alert>

      <div className="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.75fr)]">
        <div className="min-w-0 border border-border/60 bg-background p-3">
          <div className="mb-2 text-xs font-medium text-muted-foreground">
            Explain dependencies
          </div>
          <div className="grid gap-2 text-xs">
            <DependencyLine
              label="metrics"
              value={(result.required_metrics ?? []).join(", ")}
            />
            <DependencyLine
              label="marts"
              value={(result.required_marts ?? []).join(", ")}
            />
            <DependencyLine
              label="columns"
              value={formatRequiredColumns(result.required_columns)}
            />
          </div>
        </div>

        <div className="min-w-0 border border-border/60 bg-background p-3">
          <div className="mb-2 text-xs font-medium text-muted-foreground">
            RuleVersionSpec
          </div>
          <pre className="max-h-64 overflow-auto text-[11px] leading-relaxed break-all whitespace-pre-wrap text-muted-foreground">
            {JSON.stringify(ruleSpec, null, 2)}
          </pre>
        </div>
      </div>
    </div>
  )
}

function DependencyLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid min-w-0 grid-cols-[5rem_minmax(0,1fr)] gap-2">
      <div className="text-muted-foreground">{label}</div>
      <div className="truncate font-medium">{value || "--"}</div>
    </div>
  )
}

export function StrategyPage() {
  const navigate = useNavigate()
  const metricsQuery = useMetricsQuery()
  const explainMutation = useExplainMutation()
  const previewMutation = useStrategyPreviewMutation()
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
  const strategyCatalogOptions = hasRealMetricsCatalog
    ? strategyCatalog
    : indicatorCatalog
  const [activeStep, setActiveStep] = useState<Step>("indicators")
  const [conditionGroups, setConditionGroups] = useState<
    StrategyConditionGroup[]
  >([])
  const [weightIndicators, setWeightIndicators] = useState<WeightIndicator[]>(
    []
  )
  const [previewDraftWeightIndicators, setPreviewDraftWeightIndicators] =
    useState<WeightIndicator[]>(() => buildPreviewWeightIndicators([]))
  const [previewAppliedWeightIndicators, setPreviewAppliedWeightIndicators] =
    useState<WeightIndicator[]>(() => buildPreviewWeightIndicators([]))
  const [simulationSettings, setSimulationSettings] =
    useState<SimulationSettings>(defaultSimulationSettings)
  const [backtestPeriod, setBacktestPeriod] = useState<BacktestPeriod>("1y")
  const [backtestBenchmark, setBacktestBenchmark] =
    useState<BacktestBenchmark>("000300.SH")
  const [adapterError, setAdapterError] = useState<string | null>(null)
  const [lastRuleSpec, setLastRuleSpec] = useState<RuleVersionSpec | null>(null)
  const [lastExplainResult, setLastExplainResult] =
    useState<ExplainResponse | null>(null)
  const [lastExplainAt, setLastExplainAt] = useState<string | null>(null)
  const [isExplainStale, setIsExplainStale] = useState(false)
  const [previewRange, setPreviewRange] =
    useState<PreviewRange>(defaultPreviewRange)
  const [previewAdapterError, setPreviewAdapterError] = useState<string | null>(
    null
  )
  const [lastPreviewRuleSpec, setLastPreviewRuleSpec] =
    useState<RuleVersionSpec | null>(null)
  const [lastPreviewResult, setLastPreviewResult] =
    useState<StrategyPreviewResponse | null>(null)
  const [lastPreviewAt, setLastPreviewAt] = useState<string | null>(null)
  const [isPreviewStale, setIsPreviewStale] = useState(false)
  const canEditConditions = strategyCatalogOptions.length > 0
  const canExplainRule = hasRealMetricsCatalog
  const canEditWeights = hasRealScoringCatalog

  function markRuleDraftChanged() {
    setAdapterError(null)
    setPreviewAdapterError(null)
    if (lastRuleSpec || lastExplainResult || lastExplainAt) {
      setIsExplainStale(true)
    }
    if (lastPreviewRuleSpec || lastPreviewResult || lastPreviewAt) {
      setIsPreviewStale(true)
    }
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

  async function validateRuleDraft() {
    explainMutation.reset()
    setAdapterError(null)

    try {
      if (!metricsQuery.data || metricsQuery.data.length === 0) {
        throw new StrategyRuleSpecError(
          "Rearview 指标目录未加载，不能提交真实 explain"
        )
      }

      const { rule } = buildStrategySelectionRuleSpec(
        conditionGroups,
        metricsQuery.data ?? []
      )
      const result = await explainMutation.mutateAsync({ rule })
      setLastRuleSpec(rule)
      setLastExplainResult(result)
      setLastExplainAt(new Date().toISOString())
      setIsExplainStale(false)
    } catch (error) {
      if (error instanceof StrategyRuleSpecError) {
        setAdapterError(error.message)
      }
    }
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
    nextWeightIndicators: WeightIndicator[] = weightIndicators,
    options: { syncMainDraft?: boolean } = {}
  ) {
    previewMutation.reset()
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

      const requestRange = buildPreviewRequestRange(previewRange)
      const previewWeights = cloneWeightIndicators(nextWeightIndicators)
      const { rule } = buildStrategyPreviewRuleSpec(
        conditionGroups,
        previewWeights,
        metricsQuery.data,
        { topN: requestRange.top_n }
      )
      const result = await previewMutation.mutateAsync({
        rule,
        ...requestRange,
      })

      setPreviewDraftWeightIndicators(previewWeights)
      setPreviewAppliedWeightIndicators(previewWeights)
      if (options.syncMainDraft) {
        setWeightIndicators(previewWeights)
      }
      setLastPreviewRuleSpec(rule)
      setLastPreviewResult(result)
      setLastPreviewAt(new Date().toISOString())
      setIsPreviewStale(false)
      setActiveStep("preview")
    } catch (error) {
      setPreviewAdapterError(formatErrorMessage(error))
      if (error instanceof StrategyRuleSpecError) {
        return
      }
    }
  }

  function changeStep(step: Step) {
    if (step === "preview") {
      void openPreview()
      return
    }

    setActiveStep(step)
  }

  function updatePreviewDraftWeightScore(indicatorId: string, score: number) {
    setPreviewAdapterError(null)
    if (lastPreviewRuleSpec || lastPreviewResult || lastPreviewAt) {
      setIsPreviewStale(true)
    }
    setPreviewDraftWeightIndicators((current) =>
      current.map((indicator) =>
        indicator.id === indicatorId
          ? { ...indicator, score: clampScore(score) }
          : indicator
      )
    )
  }

  function updatePreviewRange(patch: Partial<PreviewRange>) {
    setPreviewAdapterError(null)
    if (lastPreviewRuleSpec || lastPreviewResult || lastPreviewAt) {
      setIsPreviewStale(true)
    }
    setPreviewRange((current) => ({
      ...current,
      ...patch,
      topN:
        patch.topN === undefined
          ? current.topN
          : Math.max(1, Math.floor(patch.topN || 1)),
    }))
  }

  function applyPreviewWeightIndicators() {
    void openPreview(previewDraftWeightIndicators, { syncMainDraft: true })
  }

  const content = stepContent[activeStep]
  const isSplitStep =
    activeStep === "preview" ||
    activeStep === "simulation" ||
    activeStep === "backtest"
  const showStepActions = activeStep !== "backtest"

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
                    usingFallback={!hasRealMetricsCatalog}
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

                  <ExplainStatusPanel
                    adapterError={adapterError}
                    error={
                      explainMutation.isError ? explainMutation.error : null
                    }
                    isPending={explainMutation.isPending}
                    lastExplainAt={lastExplainAt}
                    result={lastExplainResult}
                    ruleSpec={lastRuleSpec}
                    stale={isExplainStale}
                  />
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
                  draftWeightIndicators={previewDraftWeightIndicators}
                  error={
                    previewAdapterError ??
                    (previewMutation.isError
                      ? formatErrorMessage(previewMutation.error)
                      : null)
                  }
                  isPending={previewMutation.isPending}
                  isStale={isPreviewStale}
                  previewRange={previewRange}
                  previewResult={lastPreviewResult}
                  weightIndicators={weightIndicators}
                  onDraftWeightScoreChange={updatePreviewDraftWeightScore}
                  onPreviewRangeChange={updatePreviewRange}
                />
              ) : activeStep === "simulation" ? (
                <SimulationPositionPanel
                  appliedWeightIndicators={previewAppliedWeightIndicators}
                  conditionGroups={conditionGroups}
                  settings={simulationSettings}
                  onSettingsChange={setSimulationSettings}
                />
              ) : activeStep === "backtest" ? (
                <BacktestPanel
                  benchmark={backtestBenchmark}
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
                        className="w-full sm:w-48"
                        disabled={
                          explainMutation.isPending ||
                          !canEditConditions ||
                          !canExplainRule
                        }
                        onClick={validateRuleDraft}
                        type="button"
                      >
                        {explainMutation.isPending ? (
                          <Spinner data-icon="inline-start" />
                        ) : null}
                        校验规则
                      </Button>
                      <Button
                        variant="outline"
                        size="lg"
                        className="w-full sm:w-48"
                        disabled={!lastExplainResult || isExplainStale}
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
                        previewMutation.isPending ||
                        !canEditWeights ||
                        !hasRealMetricsCatalog
                      }
                      onClick={() => void openPreview()}
                      type="button"
                    >
                      {previewMutation.isPending ? (
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
                          onClick={() => setActiveStep("simulation")}
                          type="button"
                        >
                          模拟建仓
                        </Button>
                        <Button size="lg" variant="ghost" type="button">
                          保存草稿
                        </Button>
                      </div>
                      <div className="hidden xl:block" />
                      <Button
                        variant="outline"
                        size="lg"
                        className="w-full sm:w-48 xl:ml-2"
                        disabled={previewMutation.isPending}
                        onClick={applyPreviewWeightIndicators}
                        type="button"
                      >
                        {previewMutation.isPending ? (
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
                          onClick={() => setActiveStep("backtest")}
                          type="button"
                        >
                          <Play data-icon="inline-start" />
                          执行回测
                        </Button>
                        <Button size="lg" variant="ghost" type="button">
                          保存草稿
                        </Button>
                      </div>
                      <div className="hidden xl:block" />
                      <div className="hidden xl:block" />
                    </>
                  ) : null}
                  {activeStep !== "preview" && activeStep !== "simulation" ? (
                    <Button size="lg" variant="ghost" type="button">
                      保存草稿
                    </Button>
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
                      onClick={() =>
                        navigate("/dashboard", { viewTransition: true })
                      }
                      type="button"
                    >
                      运行策略
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
