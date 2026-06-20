import { Fragment, useEffect, useRef, useState } from "react"
import { Link, useParams } from "react-router-dom"
import { ArrowLeft, Trash2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty"
import { Separator } from "@/components/ui/separator"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { NavBenchmarkChart } from "@/components/racingline/dashboard/portfolio-overview-board"
import {
  portfolioCards,
  formatMetricValue,
  getMetricToneClassName,
  type Metric,
} from "@/components/racingline/dashboard/portfolio-data"
import { cn } from "@/lib/utils"

type DetailTradeDirection = "buy" | "hold" | "sell"

type HoldingRow = {
  code: string
  name: string
  weight: number
  cost: number
  price: number
  change: number
  contribution: number
  holdingDays: number
}

type DetailTrade = {
  changePercent: string
  contribution: string
  costPrice: string
  currentPrice: string
  direction: DetailTradeDirection
  holdingDays: string
  rebalanceReason: string
  securityCode: string
  securityName: string
}

type DetailRebalanceRecord = {
  date: string
  trades: DetailTrade[]
}

const holdingsByPortfolioId: Record<string, HoldingRow[]> = {
  "dividend-low-vol-rotation": [
    {
      code: "600036.SH",
      name: "招商银行",
      weight: 0.18,
      cost: 37.24,
      price: 39.18,
      change: 0.0521,
      contribution: 0.0094,
      holdingDays: 42,
    },
    {
      code: "601318.SH",
      name: "中国平安",
      weight: 0.16,
      cost: 48.66,
      price: 51.02,
      change: 0.0485,
      contribution: 0.0078,
      holdingDays: 31,
    },
    {
      code: "600900.SH",
      name: "长江电力",
      weight: 0.14,
      cost: 27.18,
      price: 28.05,
      change: 0.032,
      contribution: 0.0045,
      holdingDays: 56,
    },
    {
      code: "601988.SH",
      name: "中国银行",
      weight: 0.11,
      cost: 4.62,
      price: 4.74,
      change: 0.026,
      contribution: 0.0029,
      holdingDays: 23,
    },
  ],
  "growth-trend-enhanced": [
    {
      code: "688981.SH",
      name: "中芯国际",
      weight: 0.17,
      cost: 84.32,
      price: 91.08,
      change: 0.0802,
      contribution: 0.0136,
      holdingDays: 18,
    },
    {
      code: "300750.SZ",
      name: "宁德时代",
      weight: 0.16,
      cost: 246.7,
      price: 259.16,
      change: 0.0505,
      contribution: 0.0081,
      holdingDays: 21,
    },
    {
      code: "002475.SZ",
      name: "立讯精密",
      weight: 0.13,
      cost: 39.12,
      price: 41.28,
      change: 0.0552,
      contribution: 0.0072,
      holdingDays: 14,
    },
    {
      code: "600276.SH",
      name: "恒瑞医药",
      weight: 0.1,
      cost: 47.35,
      price: 45.86,
      change: -0.0315,
      contribution: -0.0032,
      holdingDays: 9,
    },
  ],
}

const detailTradeCandidates = [
  { securityCode: "600519.SH", securityName: "贵州茅台", basePrice: 1518.6 },
  { securityCode: "688981.SH", securityName: "中芯国际", basePrice: 86.4 },
  { securityCode: "300750.SZ", securityName: "宁德时代", basePrice: 248.2 },
  { securityCode: "600036.SH", securityName: "招商银行", basePrice: 38.1 },
  { securityCode: "601318.SH", securityName: "中国平安", basePrice: 49.8 },
  { securityCode: "600900.SH", securityName: "长江电力", basePrice: 27.6 },
  { securityCode: "002475.SZ", securityName: "立讯精密", basePrice: 40.4 },
  { securityCode: "600276.SH", securityName: "恒瑞医药", basePrice: 46.9 },
  { securityCode: "601988.SH", securityName: "中国银行", basePrice: 4.7 },
  { securityCode: "000333.SZ", securityName: "美的集团", basePrice: 68.5 },
]

function StrategyDetailPage() {
  const { portfolioId } = useParams()
  const [selectedRebalanceDate, setSelectedRebalanceDate] = useState("")
  const rebalanceDateScrollerRef = useRef<HTMLDivElement | null>(null)
  const portfolio = portfolioCards.find((item) => item.id === portfolioId)
  const holdings = portfolio ? (holdingsByPortfolioId[portfolio.id] ?? []) : []
  const records = portfolio
    ? buildDetailRebalanceRecords(holdings, portfolio.backtestDays)
    : []

  useEffect(() => {
    const scroller = rebalanceDateScrollerRef.current

    if (!scroller) {
      return
    }

    scroller.scrollLeft = scroller.scrollWidth
  }, [records.length])

  if (!portfolio) {
    return (
      <Empty className="min-h-[calc(100svh-8rem)] border border-dashed border-border/70">
        <EmptyHeader>
          <EmptyTitle>未找到策略</EmptyTitle>
          <EmptyDescription>该策略可能已删除或链接已经失效。</EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button
            render={<Link to="/dashboard" viewTransition />}
            nativeButton={false}
            size="lg"
          >
            返回策略看板
          </Button>
        </EmptyContent>
      </Empty>
    )
  }

  const selectedRebalanceRecord =
    records.find((record) => record.date === selectedRebalanceDate) ??
    records.at(-1)
  const selectedRebalanceTradeSections = selectedRebalanceRecord
    ? buildRebalanceTradeSections(selectedRebalanceRecord.trades)
    : []
  const latestPoint = portfolio.curve.at(-1)
  const previousPoint = portfolio.curve.at(-2)
  const latestStrategyReturn =
    latestPoint && previousPoint ? latestPoint.nav / previousPoint.nav - 1 : 0
  const latestBenchmarkReturn =
    latestPoint && previousPoint
      ? latestPoint.benchmark / previousPoint.benchmark - 1
      : 0
  const latestExcessReturn = latestPoint
    ? latestPoint.nav - latestPoint.benchmark
    : 0
  const returnMetrics = buildReturnMetrics(
    portfolio.returns,
    latestExcessReturn
  )
  const metricGroups = [
    { title: "收益指标", metrics: returnMetrics },
    { title: "风险指标", metrics: portfolio.risk },
    { title: "性价比", metrics: portfolio.efficiency },
    { title: "相对市场", metrics: portfolio.relative },
  ]

  return (
    <section className="mx-auto flex min-h-[calc(100svh-8rem)] w-full max-w-[72rem] flex-col gap-4">
      <div className="flex h-9 min-w-0 items-center gap-3">
        <Button
          render={<Link to="/dashboard" viewTransition />}
          nativeButton={false}
          variant="ghost"
          size="icon-sm"
          className="text-muted-foreground hover:bg-muted/60 hover:text-foreground"
          aria-label="返回策略看板"
        >
          <ArrowLeft />
        </Button>
        <h1 className="truncate text-lg font-medium">{portfolio.name}</h1>
        <div aria-hidden="true" className="h-5 w-px shrink-0 bg-border/90" />
        <div className="flex min-w-0 flex-1 items-center gap-3 overflow-hidden text-xs text-muted-foreground tabular-nums">
          <span className="shrink-0">建仓: {portfolio.startDate}</span>
          <span className="shrink-0">回测: {portfolio.backtestDays} 天</span>
          <span className="shrink-0">模拟: {portfolio.simulationDays} 天</span>
        </div>
        <Dialog>
          <DialogTrigger
            render={
              <Button
                variant="ghost"
                size="sm"
                type="button"
                className="shrink-0 text-muted-foreground hover:bg-muted/60 hover:text-destructive"
              />
            }
          >
            <Trash2 data-icon="inline-start" />
            删除
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>删除策略</DialogTitle>
              <DialogDescription>删除后该策略将从看板移除。</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <DialogClose render={<Button variant="outline" type="button" />}>
                取消
              </DialogClose>
              <DialogClose
                render={<Button variant="destructive" type="button" />}
              >
                确认删除
              </DialogClose>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>

      <div className="flex min-h-full flex-col gap-4 pt-2">
        <div className="flex min-h-0 flex-col gap-4">
          <div className="flex w-full flex-col gap-4">
            <section className="flex flex-col gap-3">
              <div className="flex items-baseline gap-2">
                <div className="text-sm font-medium">策略业绩</div>
              </div>

              <div className="flex flex-col gap-4">
                <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
                  <DetailSummaryMetric
                    label="业绩日期"
                    value={latestPoint?.time ?? portfolio.startDate}
                  />
                  <DetailSummaryMetric
                    label="业绩基准"
                    value="沪深300"
                    detail="000300.SH"
                  />
                  {latestPoint ? (
                    <>
                      <DetailSummaryMetric
                        label="策略净值"
                        value={portfolio.latestNav.toFixed(4)}
                        detail={formatSignedPercent(latestStrategyReturn)}
                        detailClassName={getSignedValueClassName(
                          formatSignedPercent(latestStrategyReturn)
                        )}
                      />
                      <DetailSummaryMetric
                        label="基准净值"
                        value={latestPoint.benchmark.toFixed(4)}
                        detail={formatSignedPercent(latestBenchmarkReturn)}
                        detailClassName={getSignedValueClassName(
                          formatSignedPercent(latestBenchmarkReturn)
                        )}
                      />
                    </>
                  ) : null}
                </div>
                <div className="grid gap-x-6 gap-y-3 md:grid-cols-2 xl:grid-cols-4">
                  {metricGroups.map((group) => (
                    <MetricGroup
                      key={group.title}
                      title={group.title}
                      metrics={group.metrics}
                    />
                  ))}
                </div>
              </div>
            </section>

            <Separator className="bg-border/60" />

            <section className="flex flex-col gap-3">
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
              <NavBenchmarkChart
                className="h-60"
                height={240}
                points={portfolio.curve}
              />
            </section>

            <Separator className="bg-border/60" />

            <section className="flex flex-col gap-3">
              <div className="flex items-center justify-between gap-3">
                <div className="text-sm font-medium">持仓记录</div>
                <div className="text-xs text-muted-foreground tabular-nums">
                  {records.length} 个调仓日
                </div>
              </div>

              <div
                ref={rebalanceDateScrollerRef}
                className="h-[32px] shrink-0 [scrollbar-width:thin] overflow-x-auto overflow-y-hidden overscroll-x-contain pb-3 [&::-webkit-scrollbar]:h-[2px] [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent"
              >
                <div className="flex min-w-max gap-1.5 pr-1">
                  {records.map((record) => {
                    const isSelected =
                      record.date === selectedRebalanceRecord?.date
                    const positionCount = getPositionCount(record.trades)

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
                      只 / 调出{" "}
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
                        <TableHead className="h-6 w-[17%] px-1">股票</TableHead>
                        <TableHead className="h-6 w-[17%] px-1">
                          调仓理由
                        </TableHead>
                        <TableHead className="h-6 w-[11%] px-1 text-right">
                          持仓天数
                        </TableHead>
                        <TableHead className="h-6 w-[12%] px-1 text-right">
                          涨跌幅
                        </TableHead>
                        <TableHead className="h-6 w-[14%] px-1 text-right">
                          成本价
                        </TableHead>
                        <TableHead className="h-6 w-[14%] px-1 text-right">
                          现价
                        </TableHead>
                        <TableHead className="h-6 w-[15%] px-1 text-right">
                          收益贡献
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {selectedRebalanceTradeSections.map((section) => (
                        <Fragment key={section.direction}>
                          <TableRow className="bg-muted/30 hover:bg-muted/30">
                            <TableCell
                              colSpan={7}
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
                              <TableCell className="px-1 py-0.5">
                                <span className="block truncate text-muted-foreground">
                                  {trade.rebalanceReason || "-"}
                                </span>
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
      </div>
    </section>
  )
}

function DetailSummaryMetric({
  detail,
  detailClassName,
  label,
  value,
  valueClassName,
}: {
  detail?: string
  detailClassName?: string
  label: string
  value: string
  valueClassName?: string
}) {
  return (
    <div className="min-w-0">
      <div className="truncate text-xs text-muted-foreground">{label}</div>
      <div
        className={cn(
          "mt-1 flex min-w-0 items-baseline gap-2 truncate text-sm font-medium tabular-nums"
        )}
      >
        <span className={cn("truncate", valueClassName)}>{value}</span>
        {detail ? (
          <span
            className={cn(
              "truncate text-xs font-normal text-muted-foreground tabular-nums",
              detailClassName
            )}
          >
            {detail}
          </span>
        ) : null}
      </div>
    </div>
  )
}

function MetricGroup({ metrics, title }: { metrics: Metric[]; title: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <div className="text-xs font-medium text-muted-foreground">{title}</div>
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
                getMetricToneClassName(metric)
              )}
            >
              {formatMetricValue(metric)}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

function buildReturnMetrics(metrics: Metric[], excessReturn: number): Metric[] {
  return [
    metrics[0],
    {
      label: "超额收益",
      value: excessReturn,
      kind: "percent",
      tone: excessReturn >= 0 ? "up" : "down",
    },
    ...metrics.slice(1),
    {
      label: "日胜率",
      value: 0.584,
      kind: "percent",
      tone: "neutral",
    },
  ].filter(Boolean)
}

function buildDetailRebalanceRecords(
  holdings: HoldingRow[],
  backtestDays: number
): DetailRebalanceRecord[] {
  const dates = buildTradingDates("2025-01-02", Math.min(backtestDays, 252))

  return dates.map((date, dayIndex) => {
    const buyCount = 1 + (dayIndex % 4 === 0 ? 1 : 0)
    const holdCount = Math.max(2, holdings.length + (dayIndex % 3) - 1)
    const sellCount = 1 + (dayIndex % 5 === 0 ? 1 : 0)
    const buys = Array.from({ length: buyCount }, (_, index) =>
      buildDetailTrade("buy", holdings, dayIndex, index)
    )
    const holds = Array.from({ length: holdCount }, (_, index) =>
      buildDetailTrade("hold", holdings, dayIndex, index + buyCount)
    )
    const sells = Array.from({ length: sellCount }, (_, index) =>
      buildDetailTrade("sell", holdings, dayIndex, index + buyCount + holdCount)
    )

    return {
      date,
      trades: [...buys, ...holds, ...sells],
    }
  })
}

function buildDetailTrade(
  direction: DetailTradeDirection,
  holdings: HoldingRow[],
  dayIndex: number,
  offset: number
): DetailTrade {
  const holding = holdings[(dayIndex + offset) % holdings.length]
  const fallback =
    detailTradeCandidates[
      (dayIndex * 3 + offset * 5) % detailTradeCandidates.length
    ]
  const candidate = holding
    ? {
        securityCode: holding.code,
        securityName: holding.name,
        basePrice: holding.cost,
      }
    : fallback
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
    rebalanceReason: getRebalanceReason(direction, dayIndex, offset),
    securityCode: candidate.securityCode,
    securityName: candidate.securityName,
  }
}

function getRebalanceReason(
  direction: DetailTradeDirection,
  dayIndex: number,
  offset: number
) {
  if (direction === "buy") {
    const reasons = [
      "买入信号进入Top10",
      "动量排名上升",
      "低波因子改善",
      "景气度信号增强",
    ]

    return reasons[(dayIndex + offset) % reasons.length]
  }

  if (direction === "sell") {
    const reasons = [
      "跌破MA10",
      "固定止盈触发",
      "持仓到期未达阈值",
      "排名跌出持仓池",
    ]

    return reasons[(dayIndex + offset) % reasons.length]
  }

  return ""
}

function buildRebalanceTradeSections(trades: DetailTrade[]) {
  return (["buy", "hold", "sell"] as const).map((direction) => ({
    direction,
    trades: trades.filter((trade) => trade.direction === direction),
  }))
}

function getPositionCount(trades: DetailTrade[]) {
  return trades.filter((trade) => trade.direction !== "sell").length
}

function formatTradeDirection(direction: DetailTradeDirection) {
  if (direction === "buy") {
    return "调入"
  }

  if (direction === "hold") {
    return "持有"
  }

  return "调出"
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

function formatSignedPercent(value: number) {
  const sign = value > 0 ? "+" : ""

  return `${sign}${(value * 100).toFixed(2)}%`
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

export { StrategyDetailPage }
