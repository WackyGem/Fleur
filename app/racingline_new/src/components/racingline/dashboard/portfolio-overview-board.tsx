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
  portfolioCards,
  formatChangeValue,
  formatMetricValue,
  getChangeToneClassName,
  getMetricToneClassName,
  type CurvePoint,
  type Metric,
  type PortfolioCardData,
} from "@/components/racingline/dashboard/portfolio-data"

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
  title,
  metrics,
}: {
  title: string
  metrics: Metric[]
}) {
  return (
    <section className="flex flex-col gap-2">
      <div className="text-[11px] font-medium text-muted-foreground">
        {title}
      </div>
      <div className="flex flex-col gap-1">
        {metrics.map((metric) => (
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

function PortfolioOverviewCard({
  portfolio,
}: {
  portfolio: PortfolioCardData
}) {
  return (
    <Card
      size="sm"
      className="h-full py-0 transition-colors group-hover:border-foreground/35 group-hover:bg-muted/10"
    >
      <CardHeader className="border-b border-border/70 py-4">
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-2">
            <CardTitle className="text-xl leading-none">
              {portfolio.name}
            </CardTitle>
            <div className="flex flex-wrap items-center gap-1.5">
              <Badge variant="outline">建仓: {portfolio.startDate}</Badge>
              <Badge variant="outline">回测: {portfolio.backtestDays} 天</Badge>
              <Badge variant="outline">
                模拟: {portfolio.simulationDays} 天
              </Badge>
            </div>
          </div>

          <CardAction className="static flex shrink-0 flex-col items-end gap-1">
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
                {portfolio.latestNav.toFixed(4)}
              </div>
            </div>
          </CardAction>
        </div>
      </CardHeader>

      <CardContent className="flex flex-col gap-4 py-4">
        <div className="grid gap-4">
          <MetricSection title="收益指标" metrics={portfolio.returns} />
          <MetricSection title="风险指标" metrics={portfolio.risk} />
          <MetricSection title="性价比" metrics={portfolio.efficiency} />
          <MetricSection title="相对市场" metrics={portfolio.relative} />
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
            <NavBenchmarkChart points={portfolio.curve} />
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

export function PortfolioOverviewBoard() {
  return (
    <section className="mx-auto flex min-h-[calc(100svh-8rem)] w-full max-w-[72rem] flex-col gap-4">
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
        {portfolioCards.map((portfolio) => (
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
