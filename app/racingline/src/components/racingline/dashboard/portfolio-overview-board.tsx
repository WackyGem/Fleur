import { useEffect, useRef } from "react"
import { Link } from "react-router-dom"
import { createChart, LineSeries } from "lightweight-charts"
import { ChartLineData01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Separator } from "@/components/ui/separator"
import {
  formatChangeValue,
  formatMetricValue,
  getChangeToneClassName,
  getMetricToneClassName,
  getScoreBadgeVariant,
  type CurvePoint,
  type Metric,
  type PortfolioCardData,
  type SignalStock,
} from "@/components/racingline/dashboard/portfolio-data"
import { useStrategyPortfolioDashboardQuery } from "@/api/hooks"
import type { StrategyPortfolioDashboardCard } from "@/types/rearview"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"

const VISIBLE_SIGNAL_ROW_COUNT = 5
const EMPTY_RETURN_METRICS: Metric[] = [
  { label: "持仓收益", value: null, kind: "percent", tone: "neutral" },
  { label: "超额收益", value: null, kind: "percent", tone: "neutral" },
  { label: "年化收益", value: null, kind: "percent", tone: "neutral" },
  { label: "日胜率", value: null, kind: "percent", tone: "neutral" },
]
const EMPTY_RISK_METRICS: Metric[] = [
  { label: "最大回撤", value: null, kind: "percent", tone: "down" },
  { label: "年化波动率", value: null, kind: "percent", tone: "neutral" },
  { label: "下行波动率", value: null, kind: "percent", tone: "neutral" },
]

function NavBenchmarkChart({
  className = "h-38",
  height = 152,
  points,
}: {
  className?: string
  height?: number
  points: CurvePoint[]
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const container = containerRef.current

    if (!container) {
      return
    }

    const chart = createChart(container, {
      width: container.clientWidth,
      height,
      layout: {
        background: { color: "transparent" },
        textColor: "rgba(99, 95, 89, 0.78)",
        attributionLogo: false,
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { color: "rgba(120, 114, 108, 0.10)" },
      },
      crosshair: {
        vertLine: { visible: false },
        horzLine: { visible: false },
      },
      rightPriceScale: {
        visible: false,
      },
      leftPriceScale: {
        visible: false,
      },
      timeScale: {
        visible: false,
        borderVisible: false,
      },
      handleScroll: false,
      handleScale: false,
    })

    const navSeries = chart.addSeries(LineSeries, {
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

    navSeries.setData(
      points.map((point) => ({ time: point.time, value: point.nav }))
    )
    benchmarkSeries.setData(
      points.map((point) => ({ time: point.time, value: point.benchmark }))
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
  }, [height, points])

  return <div ref={containerRef} className={`${className} w-full`} />
}

function MetricSection({
  emptyMetrics,
  title,
  metrics,
}: {
  emptyMetrics: Metric[]
  title: string
  metrics: Metric[]
}) {
  const displayMetrics = metrics.length > 0 ? metrics : emptyMetrics

  return (
    <section className="flex flex-col gap-2">
      <div className="text-[11px] font-medium text-muted-foreground">
        {title}
      </div>
      <div className="flex flex-col gap-1">
        {displayMetrics.map((metric) => (
          <div
            key={metric.label}
            className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3 border-b border-border/50 py-1 last:border-b-0 last:pb-0"
          >
            <div className="text-xs text-muted-foreground">{metric.label}</div>
            <div
              className={`text-xs font-medium ${getMetricToneClassName(metric)}`}
            >
              {formatMetricValue(metric)}
            </div>
          </div>
        ))}
      </div>
    </section>
  )
}

function TodaySignalSection({
  signalDate,
  stocks,
}: {
  signalDate: string | null
  stocks: SignalStock[]
}) {
  const placeholderCount = Math.max(0, VISIBLE_SIGNAL_ROW_COUNT - stocks.length)

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[11px] font-medium text-muted-foreground">
          买入信号
        </div>
        {signalDate ? (
          <div className="text-[11px] text-muted-foreground tabular-nums">
            {formatDisplayDate(signalDate)}
          </div>
        ) : null}
      </div>
      <div className="max-h-[11rem] min-h-0 overflow-y-auto">
        <Table className="w-full table-fixed text-xs">
          <TableHeader>
            <TableRow className="hover:bg-transparent">
              <TableHead className="h-7 px-1">股票</TableHead>
              <TableHead className="h-7 w-16 px-1 text-right">得分</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {stocks.map((stock) => (
              <TableRow key={stock.code}>
                <TableCell className="px-1 py-1">
                  <div className="grid min-w-0 grid-cols-[4.5em_minmax(0,1fr)] items-center gap-1">
                    <span className="truncate font-medium">{stock.name}</span>
                    <span className="truncate text-muted-foreground tabular-nums">
                      {stock.code}
                    </span>
                  </div>
                </TableCell>
                <TableCell className="px-1 py-1 text-right">
                  <Badge variant={getScoreBadgeVariant(stock.score)}>
                    {stock.score.toFixed(1)}
                  </Badge>
                </TableCell>
              </TableRow>
            ))}
            {Array.from({ length: placeholderCount }, (_, index) => (
              <TableRow key={`placeholder-${index}`}>
                <TableCell className="px-1 py-1 text-muted-foreground">
                  --
                </TableCell>
                <TableCell className="px-1 py-1 text-right text-muted-foreground">
                  --
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </section>
  )
}

function PortfolioOverviewCard({
  portfolio,
}: {
  portfolio: PortfolioCardData
}) {
  const isPendingFirstRun = portfolio.liveStatus === "pending_first_run"

  return (
    <Card
      size="sm"
      className="h-full py-0 transition-colors group-hover:border-foreground/35 group-hover:bg-muted/10"
    >
      <CardHeader className="grid-cols-[minmax(0,1fr)_auto] gap-3 border-b border-border/70 py-4">
        <div className="flex min-w-0 flex-col gap-2">
          <CardTitle
            className="truncate text-xl leading-tight"
            title={portfolio.name}
          >
            {portfolio.name}
          </CardTitle>
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <span>建仓: {portfolio.startDate}</span>
            <span>运行: {portfolio.simulationDays} 天</span>
          </div>
        </div>

        <CardAction className="flex shrink-0 flex-col items-end gap-1">
          <div className="text-[11px] text-muted-foreground">最新净值</div>
          <div className="flex items-end justify-end gap-2">
            <div
              className={`text-xs leading-none font-medium tabular-nums ${getChangeToneClassName(
                portfolio.recentChange
              )}`}
            >
              {formatChangeValue(portfolio.recentChange)}
            </div>
            <div className="text-xl leading-none font-medium tabular-nums">
              {isPendingFirstRun
                ? "待建仓"
                : portfolio.latestNav === null
                  ? "--"
                  : portfolio.latestNav.toFixed(4)}
            </div>
          </div>
        </CardAction>
      </CardHeader>

      <CardContent className="flex flex-col gap-4 py-4">
        <div className="grid gap-4">
          <MetricSection
            emptyMetrics={EMPTY_RETURN_METRICS}
            title="收益指标"
            metrics={portfolio.returns}
          />
          <MetricSection
            emptyMetrics={EMPTY_RISK_METRICS}
            title="风险指标"
            metrics={portfolio.risk}
          />
          <TodaySignalSection
            signalDate={portfolio.signalDate}
            stocks={portfolio.todaySignals}
          />
        </div>

        <Separator />

        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between gap-3">
            <div className="text-[11px] font-medium text-muted-foreground">
              净值与基准
            </div>
            <div className="flex items-center gap-3 text-[11px] text-muted-foreground">
              <div className="inline-flex items-center gap-1.5">
                <span className="size-2 bg-primary" />
                净值
              </div>
              <div className="inline-flex items-center gap-1.5">
                <span className="size-2 bg-muted-foreground" />
                基准
              </div>
            </div>
          </div>

          <div className="overflow-hidden rounded-md border border-border/70 bg-muted/15 px-2 py-2">
            {portfolio.curve.length > 0 ? (
              <NavBenchmarkChart points={portfolio.curve} />
            ) : (
              <div className="flex h-38 items-center justify-center text-xs text-muted-foreground">
                净值曲线暂不可用
              </div>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

function CreatePortfolioCard() {
  return (
    <Link
      to="/strategies"
      viewTransition
      className="group flex min-h-[34rem] flex-col items-center justify-center gap-4 border border-dashed border-border/80 bg-muted/10 px-6 py-8 text-center transition-colors hover:border-foreground/50 hover:bg-muted/25 focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
      aria-label="前往选股页面创建新的策略组合"
    >
      <div className="flex size-10 items-center justify-center border border-dashed border-border/90 text-muted-foreground transition-colors group-hover:border-foreground/50 group-hover:text-foreground">
        <HugeiconsIcon icon={ChartLineData01Icon} />
      </div>
      <div className="flex flex-col gap-1">
        <div className="text-sm font-medium text-foreground">
          创建新的策略组合
        </div>
        <div className="text-xs text-muted-foreground">
          点击选股按钮开始构建组合
        </div>
      </div>
    </Link>
  )
}

function daysBetween(startDate: string, endDate: string) {
  const start = new Date(`${startDate}T00:00:00Z`).getTime()
  const end = new Date(`${endDate}T00:00:00Z`).getTime()

  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) {
    return 0
  }

  return Math.floor((end - start) / 86_400_000) + 1
}

function mapStrategyPortfolioCard(
  card: StrategyPortfolioDashboardCard
): PortfolioCardData {
  const latestCurvePoint = card.curve.at(-1)
  const signals =
    card.live_status === "pending_first_run"
      ? card.pending_buy_signals
      : card.today_signals

  return {
    id: card.strategy_portfolio_id,
    name: card.name,
    liveStatus: card.live_status,
    startDate: card.live_start_date,
    backtestDays: daysBetween(card.source_start_date, card.source_end_date),
    simulationDays:
      card.live_status === "pending_first_run" || !latestCurvePoint
        ? 0
        : daysBetween(card.live_start_date, latestCurvePoint.time),
    latestNav: card.latest_nav ?? null,
    recentChange: card.recent_change ?? null,
    returns: card.returns.map(mapDashboardMetric),
    risk: card.risk.map(mapDashboardMetric),
    efficiency: card.efficiency.map(mapDashboardMetric),
    relative: card.relative.map(mapDashboardMetric),
    signalDate:
      card.live_status === "pending_first_run"
        ? (signals[0]?.signal_date ?? card.initial_signal_date)
        : (latestCurvePoint?.time ?? null),
    todaySignals: signals.map((signal) => ({
      code: signal.code,
      executionDate: signal.execution_date,
      name: signal.name,
      rank: signal.rank,
      score: signal.score,
      signalDate: signal.signal_date,
    })),
    curve: card.curve,
  }
}

function formatDisplayDate(date: string) {
  return date.replaceAll("-", "/")
}

function mapDashboardMetric(
  metric: StrategyPortfolioDashboardCard["returns"][number]
): Metric {
  return {
    kind: metric.kind,
    label: metric.label,
    tone: metric.tone,
    value: metric.value ?? null,
  }
}

export function PortfolioOverviewBoard() {
  const dashboardQuery = useStrategyPortfolioDashboardQuery()
  const portfolios =
    dashboardQuery.data?.portfolios.map(mapStrategyPortfolioCard) ?? []

  return (
    <section className="mx-auto flex min-h-[calc(100svh-8rem)] w-full max-w-[88rem] flex-col gap-4">
      <div className="flex h-9 items-center gap-4">
        <h1 className="text-lg font-medium">策略看板</h1>
        <div aria-hidden="true" className="h-5 w-px shrink-0 bg-border/90" />
        <Button
          render={<Link to="/strategies" viewTransition />}
          nativeButton={false}
          variant="default"
          size="lg"
          className="h-9 bg-foreground px-4 text-sm text-background hover:bg-foreground/90"
        >
          <HugeiconsIcon icon={ChartLineData01Icon} data-icon="inline-start" />
          选股
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 2xl:grid-cols-4">
        {dashboardQuery.isLoading ? (
          <div className="flex min-h-[34rem] items-center justify-center border border-border/70 bg-muted/10 px-6 py-8 text-center text-sm text-muted-foreground">
            策略组合加载中
          </div>
        ) : null}
        {dashboardQuery.isError ? (
          <div className="flex min-h-[34rem] items-center justify-center border border-border/70 bg-muted/10 px-6 py-8 text-center text-sm text-muted-foreground">
            策略组合加载失败
          </div>
        ) : null}
        {portfolios.map((portfolio) => (
          <Link
            key={portfolio.id}
            to={`/dashboard/strategies/${portfolio.id}`}
            viewTransition
            className="group block h-full focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
            aria-label={`查看${portfolio.name}策略详情`}
          >
            <PortfolioOverviewCard portfolio={portfolio} />
          </Link>
        ))}
        <CreatePortfolioCard />
      </div>
    </section>
  )
}

export { NavBenchmarkChart }
