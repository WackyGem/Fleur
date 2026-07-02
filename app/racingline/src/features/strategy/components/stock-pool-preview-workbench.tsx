import { useEffect, useMemo, useRef, useState, type ReactNode } from "react"
import {
  CandlestickSeries,
  HistogramSeries,
  LineSeries,
  createChart,
} from "lightweight-charts"

import {
  usePreviewChartContextQuery,
  useStrategyPreviewPoolPageQuery,
} from "@/api/hooks"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty"
import { Input } from "@/components/ui/input"
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
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import {
  buildPreviewPresentation,
  buildPreviewStockRow,
  formatSecurityBoard,
  type PreviewSnapshot,
  type PreviewStockRow,
  type PreviewTradeDateRow,
  type PreviewValueRow,
} from "@/features/strategy/preview"
import type {
  IndicatorCatalog,
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import { StrategySplitPanel } from "@/features/strategy/components/strategy-split-panel"
import { WeightScoreSlider } from "@/features/strategy/components/weight-score-slider"
import {
  clampScore,
  formatWeightIndicator,
} from "@/features/strategy/utils"
import type {
  PreviewChartContextResponse,
  PreviewChartContextSeriesRow,
} from "@/types/rearview"

type StockPoolPreviewWorkbenchProps = {
  appliedWeightIndicators: WeightIndicator[]
  catalogOptions: IndicatorCatalog[]
  conditionGroups: StrategyConditionGroup[]
  hasStrategyInput: boolean
  onUpdateWeightIndicator: (
    indicatorId: string,
    patch: Partial<WeightIndicator>
  ) => void
  previewSnapshot?: PreviewSnapshot | null
  weightIndicators: WeightIndicator[]
}

const adjustmentOptions = [
  { label: "除权", value: "unadjusted" },
  { label: "前复权", value: "forward_adjusted" },
  { label: "后复权", value: "backward_adjusted" },
] as const

const trendLineOptions = ["MA5", "MA10", "MA30"] as const

const pageSize = 10
const maWindows = "5,10,30"

const priceFormatter = new Intl.NumberFormat("zh-CN", {
  maximumFractionDigits: 2,
  minimumFractionDigits: 2,
})

const compactFormatter = new Intl.NumberFormat("zh-CN", {
  maximumFractionDigits: 1,
})

const percentFormatter = new Intl.NumberFormat("zh-CN", {
  maximumFractionDigits: 2,
  minimumFractionDigits: 2,
  style: "percent",
})

function StockPoolPreviewWorkbench({
  catalogOptions,
  hasStrategyInput,
  onUpdateWeightIndicator,
  previewSnapshot,
  weightIndicators,
}: StockPoolPreviewWorkbenchProps) {
  const [selectedTradeDate, setSelectedTradeDate] = useState("")
  const [selectedSecurityCode, setSelectedSecurityCode] = useState("")
  const [poolPageState, setPoolPageState] = useState({
    offset: 0,
    previewId: "",
    tradeDate: "",
  })
  const [adjustmentMode, setAdjustmentMode] =
    useState<(typeof adjustmentOptions)[number]["value"]>("forward_adjusted")
  const presentation = useMemo(
    () => (previewSnapshot ? buildPreviewPresentation(previewSnapshot) : null),
    [previewSnapshot]
  )
  const dailyStockPools = useMemo(
    () => presentation?.tradeDates ?? [],
    [presentation]
  )
  const effectiveSelectedTradeDate = dailyStockPools.some(
    (pool) => pool.date === selectedTradeDate
  )
    ? selectedTradeDate
    : dailyStockPools.at(-1)?.date
  const selectedDailyPool =
    dailyStockPools.find((pool) => pool.date === effectiveSelectedTradeDate) ??
    null
  const latestChartTradeDate =
    dailyStockPools.at(-1)?.date ?? selectedDailyPool?.date ?? ""
  const selectedPreviewId = previewSnapshot?.previewId ?? ""
  const selectedPoolDate = selectedDailyPool?.date ?? ""
  const poolOffset =
    poolPageState.previewId === selectedPreviewId &&
    poolPageState.tradeDate === selectedPoolDate
      ? poolPageState.offset
      : 0
  const hasLocalPoolPage =
    poolOffset === 0 && (selectedDailyPool?.stocks.length ?? 0) > 0
  const shouldFetchPoolPage = Boolean(
    previewSnapshot && selectedDailyPool && !hasLocalPoolPage
  )

  const poolPageRequest =
    shouldFetchPoolPage && previewSnapshot && selectedDailyPool
      ? {
          rule: previewSnapshot.appliedRuleSpec,
          trade_date: selectedDailyPool.date,
          limit: pageSize,
          offset: poolOffset,
          sort: "score_desc" as const,
        }
      : null
  const poolPageQuery = useStrategyPreviewPoolPageQuery(
    shouldFetchPoolPage ? (previewSnapshot?.previewId ?? null) : null,
    poolPageRequest
  )
  const pagedStocks = useMemo(() => {
    if (!previewSnapshot || !poolPageQuery.data) {
      return selectedDailyPool?.stocks ?? []
    }

    return poolPageQuery.data.items.map((signal) =>
      buildPreviewStockRow(signal, previewSnapshot.labels)
    )
  }, [poolPageQuery.data, previewSnapshot, selectedDailyPool?.stocks])
  const selectedStock =
    pagedStocks.find((stock) => stock.code === selectedSecurityCode) ??
    selectedDailyPool?.stocks.find(
      (stock) => stock.code === selectedSecurityCode
    ) ??
    pagedStocks[0] ??
    selectedDailyPool?.stocks[0] ??
    null

  const analysisRequest =
    previewSnapshot && selectedDailyPool && selectedStock
      ? {
          adjustment: adjustmentMode,
          lookback_trading_days: 240,
          ma_windows: maWindows,
          security_code: selectedStock.code,
          trade_date: latestChartTradeDate,
        }
      : null
  const analysisQuery = usePreviewChartContextQuery(
    previewSnapshot?.previewId ?? null,
    analysisRequest
  )

  if (!previewSnapshot) {
    return (
      <Empty className="h-full border border-border/60">
        <EmptyHeader>
          <EmptyTitle>尚未执行股池预览</EmptyTitle>
          <EmptyDescription>
            当前页面只展示 Rearview 返回的真实预览结果。
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  }

  if (!selectedDailyPool) {
    return (
      <Empty className="h-full border border-border/60">
        <EmptyHeader>
          <EmptyTitle>股池为空</EmptyTitle>
          <EmptyDescription>当前预览没有返回交易日结果。</EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  }

  return (
    <StrategySplitPanel
      className="h-full min-h-[46rem] xl:min-h-0"
      data-has-strategy-input={hasStrategyInput}
      data-preview-stale={previewSnapshot.stale}
      mainClassName="gap-3"
      asideClassName="xl:h-full"
      main={
        <>
          <KLinePanel
            adjustmentMode={adjustmentMode}
            analysis={analysisQuery.data ?? null}
            error={analysisQuery.isError ? analysisQuery.error : null}
            isPending={analysisQuery.isPending}
            onAdjustmentModeChange={setAdjustmentMode}
            selectedPoolDate={selectedDailyPool.date}
            stock={selectedStock}
          />
          <Separator />
          <DailyStockPoolPanel
            dailyStockPools={dailyStockPools}
            onNextPage={() =>
              setPoolPageState({
                offset: poolOffset + pageSize,
                previewId: selectedPreviewId,
                tradeDate: selectedPoolDate,
              })
            }
            onPreviousPage={() =>
              setPoolPageState({
                offset: Math.max(0, poolOffset - pageSize),
                previewId: selectedPreviewId,
                tradeDate: selectedPoolDate,
              })
            }
            onSelectedDateChange={(date) => {
              setSelectedTradeDate(date)
              setSelectedSecurityCode("")
            }}
            onSelectedStockChange={setSelectedSecurityCode}
            pageHasMore={
              poolPageQuery.data?.has_more ??
              (hasLocalPoolPage
                ? selectedDailyPool.poolCount > selectedDailyPool.stocks.length
                : false)
            }
            pageOffset={poolOffset}
            pagedStocks={pagedStocks}
            selectedDate={selectedDailyPool.date}
            selectedPool={selectedDailyPool}
            selectedSecurityCode={selectedStock?.code ?? ""}
            isPoolPagePending={shouldFetchPoolPage && poolPageQuery.isPending}
          />
        </>
      }
      aside={
        <KeyDataPanel
          analysis={analysisQuery.data ?? null}
          analysisError={analysisQuery.isError ? analysisQuery.error : null}
          catalogOptions={catalogOptions}
          isAnalysisPending={analysisQuery.isPending}
          onUpdateWeightIndicator={onUpdateWeightIndicator}
          stock={selectedStock}
          weightIndicators={weightIndicators}
        />
      }
    />
  )
}

function KLinePanel({
  adjustmentMode,
  analysis,
  error,
  isPending,
  onAdjustmentModeChange,
  selectedPoolDate,
  stock,
}: {
  adjustmentMode: (typeof adjustmentOptions)[number]["value"]
  analysis: PreviewChartContextResponse | null
  error: unknown
  isPending: boolean
  onAdjustmentModeChange: (
    value: (typeof adjustmentOptions)[number]["value"]
  ) => void
  selectedPoolDate: string
  stock: PreviewStockRow | null
}) {
  const [trendLines, setTrendLines] = useState<string[]>([
    "MA5",
    "MA10",
    "MA30",
  ])
  const adjustmentLabel =
    adjustmentOptions.find((option) => option.value === adjustmentMode)
      ?.label ?? "前复权"
  const boardLabel =
    stock?.boardLabel ?? formatSecurityBoard(analysis?.security_board)
  const maAvailable = (analysis?.chart.ma?.available_windows.length ?? 0) > 0
  const visibleTrendLines = maAvailable ? trendLines : []

  return (
    <Card
      size="sm"
      className="min-h-[27rem] shrink-0 bg-transparent py-0 ring-0 sm:min-h-[28rem] xl:min-h-[29rem] xl:pr-4"
    >
      <CardHeader className="grid gap-2 px-0 pt-2 pb-1 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-start">
        <div className="min-h-[3.75rem] min-w-0">
          <CardTitle className="flex h-[3.75rem] min-w-0 flex-col justify-between group-data-[size=sm]/card:text-xl">
            <span className="flex h-7 items-center truncate leading-7">
              {stock?.name ??
                analysis?.security_name ??
                analysis?.security_code ??
                "-"}
            </span>
            <span className="flex h-7 items-center text-sm leading-5 font-normal text-muted-foreground tabular-nums">
              {formatStockSubtitle(
                stock?.code ?? analysis?.security_code,
                boardLabel
              )}
            </span>
          </CardTitle>
        </div>

        <div className="grid h-[3.75rem] min-w-0 justify-items-start gap-1 sm:justify-items-end">
          <div className="grid h-7 grid-cols-[2.5rem_auto] items-center gap-2">
            <span className="text-right text-xs text-muted-foreground">
              复权
            </span>
            <Select
              value={adjustmentMode}
              onValueChange={(value) => {
                if (value) {
                  onAdjustmentModeChange(
                    value as (typeof adjustmentOptions)[number]["value"]
                  )
                }
              }}
            >
              <SelectTrigger
                size="sm"
                className="min-w-20 shrink-0 bg-background"
              >
                <SelectValue>
                  <span>{adjustmentLabel}</span>
                </SelectValue>
              </SelectTrigger>
              <SelectContent align="end" className="bg-background">
                <SelectGroup>
                  {adjustmentOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>

          <div className="grid h-7 grid-cols-[2.5rem_auto] items-center gap-2">
            <span className="text-right text-xs text-muted-foreground">
              趋势线
            </span>
            <ToggleGroup
              multiple
              value={visibleTrendLines}
              onValueChange={(nextTrendLines) => setTrendLines(nextTrendLines)}
              variant="outline"
              size="sm"
              spacing={0}
              className="min-w-0 flex-wrap justify-end"
              disabled={!maAvailable}
            >
              {trendLineOptions.map((option) => (
                <ToggleGroupItem
                  key={option}
                  value={option}
                  className="text-muted-foreground/70 aria-pressed:text-foreground"
                  disabled={!maAvailable}
                >
                  {option}
                </ToggleGroupItem>
              ))}
            </ToggleGroup>
          </div>
        </div>
      </CardHeader>

      <CardContent className="px-0 pt-0 pb-3">
        <div className="h-[22rem] overflow-hidden border border-border/70 bg-muted/10 sm:h-[23rem] xl:h-[24rem]">
          {isPending ? (
            <div className="flex h-full items-center justify-center">
              <Spinner />
            </div>
          ) : error ? (
            <div className="flex h-full items-center p-3">
              <Alert variant="destructive">
                <AlertTitle>个股上下文加载失败</AlertTitle>
                <AlertDescription>{formatErrorMessage(error)}</AlertDescription>
              </Alert>
            </div>
          ) : analysis && analysis.chart.series.length > 0 ? (
            <CandlestickChart
              selectedPoolDate={selectedPoolDate}
              series={analysis.chart.series}
              visibleTrendLines={visibleTrendLines}
            />
          ) : (
            <Empty className="h-full">
              <EmptyHeader>
                <EmptyTitle>暂无行情序列</EmptyTitle>
                <EmptyDescription>
                  Rearview 没有返回当前证券的图表数据。
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          )}
        </div>
      </CardContent>
    </Card>
  )
}

function CandlestickChart({
  selectedPoolDate,
  series,
  visibleTrendLines,
}: {
  selectedPoolDate: string
  series: PreviewChartContextSeriesRow[]
  visibleTrendLines: string[]
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const [hoveredRow, setHoveredRow] =
    useState<PreviewChartContextSeriesRow | null>(null)
  const [dateMarkerLeft, setDateMarkerLeft] = useState<number | null>(null)
  const latestDisplayRow = useMemo(
    () =>
      [...series]
        .reverse()
        .find((row) => row.ohlc || typeof row.volume === "number") ?? null,
    [series]
  )
  const displayedRow = hoveredRow ?? latestDisplayRow

  useEffect(() => {
    const container = containerRef.current

    if (!container) {
      return
    }

    const chart = createChart(container, {
      width: container.clientWidth,
      height: Math.max(container.clientHeight, 180),
      layout: {
        attributionLogo: false,
        background: { color: "transparent" },
        textColor: "rgba(87, 80, 72, 0.82)",
      },
      grid: {
        horzLines: { color: "rgba(120, 114, 108, 0.12)" },
        vertLines: { color: "rgba(120, 114, 108, 0.08)" },
      },
      rightPriceScale: {
        borderVisible: false,
        scaleMargins: {
          bottom: 0.28,
          top: 0.08,
        },
      },
      timeScale: {
        borderVisible: false,
        fixLeftEdge: true,
        fixRightEdge: true,
      },
    })

    const candleSeries = chart.addSeries(CandlestickSeries, {
      borderVisible: false,
      downColor: "#2f8f57",
      upColor: "#bd4a3a",
      wickDownColor: "#2f8f57",
      wickUpColor: "#bd4a3a",
    })

    candleSeries.setData(
      series
        .filter((row) => row.ohlc)
        .map((row) => ({
          close: row.ohlc?.close ?? 0,
          high: row.ohlc?.high ?? 0,
          low: row.ohlc?.low ?? 0,
          open: row.ohlc?.open ?? 0,
          time: row.trade_date,
        }))
    )

    const volumeSeries = chart.addSeries(HistogramSeries, {
      priceFormat: {
        type: "volume",
      },
      priceLineVisible: false,
      priceScaleId: "",
    })
    volumeSeries.priceScale().applyOptions({
      scaleMargins: {
        bottom: 0,
        top: 0.72,
      },
    })
    volumeSeries.setData(
      series
        .filter((row) => typeof row.volume === "number")
        .map((row) => ({
          color:
            row.ohlc && row.ohlc.close < row.ohlc.open
              ? "rgba(47, 143, 87, 0.34)"
              : "rgba(189, 74, 58, 0.34)",
          time: row.trade_date,
          value: row.volume ?? 0,
        }))
    )

    for (const trendLine of visibleTrendLines) {
      const window = trendLine.replace("MA", "")
      const color = maLineColor(trendLine)
      const lineSeries = chart.addSeries(LineSeries, {
        color,
        lineWidth: 1,
        priceLineVisible: false,
      })
      lineSeries.setData(
        series
          .map((row) => {
            const value =
              row.ma?.[window]

            return typeof value === "number"
              ? {
                  time: row.trade_date,
                  value,
                }
              : null
          })
          .filter((row): row is { time: string; value: number } => row !== null)
      )
    }
    chart.timeScale().fitContent()

    const rowsByDate = new Map(series.map((row) => [row.trade_date, row]))
    const updateDateMarker = () => {
      if (!selectedPoolDate) {
        setDateMarkerLeft(null)
        return
      }

      const coordinate = chart.timeScale().timeToCoordinate(selectedPoolDate)
      setDateMarkerLeft(
        typeof coordinate === "number" && Number.isFinite(coordinate)
          ? coordinate
          : null
      )
    }
    updateDateMarker()
    chart.timeScale().subscribeVisibleLogicalRangeChange(updateDateMarker)
    chart.subscribeCrosshairMove((param) => {
      if (!param.time) {
        setHoveredRow(null)
        return
      }

      const tradeDate = chartTimeToDateKey(param.time)
      setHoveredRow(tradeDate ? (rowsByDate.get(tradeDate) ?? null) : null)
    })

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0]

      if (!entry) {
        return
      }

      chart.applyOptions({
        height: Math.max(entry.contentRect.height, 180),
        width: entry.contentRect.width,
      })
      updateDateMarker()
    })

    resizeObserver.observe(container)

    return () => {
      chart.timeScale().unsubscribeVisibleLogicalRangeChange(updateDateMarker)
      resizeObserver.disconnect()
      chart.remove()
    }
  }, [selectedPoolDate, series, visibleTrendLines])

  return (
    <div className="relative h-full min-h-[12rem] w-full">
      <div ref={containerRef} className="h-full min-h-[12rem] w-full" />
      {dateMarkerLeft !== null ? (
        <div
          aria-hidden
          className="pointer-events-none absolute top-0 z-10 h-0 w-0 -translate-x-1/2 border-x-[6px] border-t-[8px] border-x-transparent border-t-[#9a4f12]"
          data-chart-date-marker={selectedPoolDate}
          style={{ left: dateMarkerLeft }}
        />
      ) : null}
      {displayedRow ? (
        <div
          className="pointer-events-none absolute top-0 left-0 z-10 w-[7.25rem] border border-border/70 bg-background/92 px-1.5 py-1 shadow-sm backdrop-blur"
          data-chart-hover-row={displayedRow.trade_date}
        >
          <div className="border-b border-border/50 pb-0.5 text-[10px] leading-3 font-medium tabular-nums">
            {displayedRow.trade_date}
          </div>
          <div className="mt-1 grid gap-0.5 text-[9px] leading-3 text-muted-foreground tabular-nums">
            <ChartHoverRow label="开盘" value={formatPrice(displayedRow.ohlc?.open)} />
            <ChartHoverRow label="最高" value={formatPrice(displayedRow.ohlc?.high)} />
            <ChartHoverRow label="最低" value={formatPrice(displayedRow.ohlc?.low)} />
            <ChartHoverRow label="收盘" value={formatPrice(displayedRow.ohlc?.close)} />
            <ChartHoverRow label="成交量" value={formatHoverVolume(displayedRow.volume)} />
          </div>
        </div>
      ) : null}
    </div>
  )
}

function ChartHoverRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[2.5rem_minmax(0,1fr)] gap-1">
      <span>{label}</span>
      <span className="truncate text-right text-foreground">{value}</span>
    </div>
  )
}

function chartTimeToDateKey(time: unknown) {
  if (typeof time === "string") {
    return time
  }

  if (
    time &&
    typeof time === "object" &&
    "year" in time &&
    "month" in time &&
    "day" in time
  ) {
    const value = time as { day: number; month: number; year: number }
    return `${value.year}-${String(value.month).padStart(2, "0")}-${String(
      value.day
    ).padStart(2, "0")}`
  }

  return null
}

function maLineColor(trendLine: string) {
  if (trendLine === "MA5") {
    return "#7c6f64"
  }

  if (trendLine === "MA10") {
    return "#2563eb"
  }

  return "#a855f7"
}

function DailyStockPoolPanel({
  dailyStockPools,
  isPoolPagePending,
  onNextPage,
  onPreviousPage,
  onSelectedDateChange,
  onSelectedStockChange,
  pageHasMore,
  pageOffset,
  pagedStocks,
  selectedDate,
  selectedPool,
  selectedSecurityCode,
}: {
  dailyStockPools: PreviewTradeDateRow[]
  isPoolPagePending: boolean
  onNextPage: () => void
  onPreviousPage: () => void
  onSelectedDateChange: (date: string) => void
  onSelectedStockChange: (securityCode: string) => void
  pageHasMore: boolean
  pageOffset: number
  pagedStocks: PreviewStockRow[]
  selectedDate: string
  selectedPool: PreviewTradeDateRow
  selectedSecurityCode: string
}) {
  const timelineScrollRef = useRef<HTMLDivElement | null>(null)
  const latestDate = dailyStockPools.at(-1)?.date ?? ""

  useEffect(() => {
    const container = timelineScrollRef.current
    if (!container || selectedDate !== latestDate) {
      return
    }

    const frameId = window.requestAnimationFrame(() => {
      container.scrollLeft = container.scrollWidth
    })

    return () => window.cancelAnimationFrame(frameId)
  }, [latestDate, selectedDate, dailyStockPools.length])

  return (
    <Card
      size="sm"
      className="min-h-0 flex-1 bg-transparent py-0 ring-0 xl:pr-4"
    >
      <CardContent className="flex h-full min-h-0 flex-col gap-2 px-0 pt-0 pb-0">
        <div
          ref={timelineScrollRef}
          className="h-[32px] shrink-0 [scrollbar-width:thin] overflow-x-auto overflow-y-hidden overscroll-x-contain pb-3 [&::-webkit-scrollbar]:h-[2px] [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent"
        >
          <div className="flex min-w-max gap-1.5 pr-1">
            {dailyStockPools.map((pool) => {
              const isSelected = pool.date === selectedDate

              return (
                <Button
                  key={pool.date}
                  aria-label={`${pool.date} 股池 ${pool.poolCount} 只`}
                  aria-pressed={isSelected}
                  data-state={isSelected ? "selected" : "idle"}
                  className="h-[18px] w-[4.25rem] shrink-0 items-center gap-1 px-1 text-muted-foreground hover:bg-muted data-[state=selected]:bg-muted data-[state=selected]:text-foreground"
                  size="sm"
                  type="button"
                  variant="ghost"
                  onClick={() => onSelectedDateChange(pool.date)}
                >
                  <span className="text-[11px] leading-none tabular-nums">
                    {formatCompactDate(pool.date)}
                  </span>
                  <span className="text-[10px] leading-none font-normal tabular-nums opacity-80">
                    {pool.poolCount}只
                  </span>
                </Button>
              )
            })}
          </div>
        </div>

        <div className="grid h-[18px] shrink-0 gap-1 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
          <div className="min-w-0">
            <div className="text-sm font-medium tabular-nums">
              {selectedPool.date}
            </div>
          </div>
          <div className="text-xs text-muted-foreground tabular-nums">
            {selectedPool.poolCount} 只
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto">
          <Table className="min-w-[56rem] table-fixed">
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead className="h-7 w-8 px-1 text-right">#</TableHead>
                <TableHead className="h-7 w-44 px-1">股票</TableHead>
                <TableHead className="h-7 w-52 px-1">指标筛选</TableHead>
                <TableHead className="h-7 px-1">得分项</TableHead>
                <TableHead className="h-7 w-28 px-1 text-right">得分</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isPoolPagePending && pagedStocks.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={5} className="py-3">
                    <Skeleton className="h-8 w-full" />
                  </TableCell>
                </TableRow>
              ) : null}
              {pagedStocks.map((stock) => (
                <TableRow
                  key={stock.code}
                  aria-selected={stock.code === selectedSecurityCode}
                  data-state={
                    stock.code === selectedSecurityCode ? "selected" : "idle"
                  }
                  className="cursor-pointer data-[state=selected]:bg-muted/70"
                  onClick={() => onSelectedStockChange(stock.code)}
                >
                  <TableCell className="px-1 py-1 text-right tabular-nums">
                    {stock.rank}
                  </TableCell>
                  <TableCell className="px-1 py-1">
                    <div className="grid min-w-0 grid-cols-[4.5em_minmax(0,1fr)] items-center gap-1">
                      <span className="truncate font-medium">{stock.name}</span>
                      <span className="truncate text-muted-foreground tabular-nums">
                        {stock.code}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell className="px-1 py-1">
                    <InlineValueRows rows={stock.filterMetricRows} />
                  </TableCell>
                  <TableCell className="px-1 py-1">
                    <div
                      className="w-full truncate text-muted-foreground tabular-nums"
                      title={formatScoreItems(stock.scoreItems)}
                    >
                      {formatScoreItems(stock.scoreItems)}
                    </div>
                  </TableCell>
                  <TableCell
                    className="px-1 py-1 text-right"
                    title={formatScoreItems(stock.scoreItems)}
                  >
                    <span className="text-xs font-medium tabular-nums">
                      {stock.score.toFixed(1)}
                    </span>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <div className="grid shrink-0 grid-cols-[auto_1fr_auto] items-center gap-2 pb-1">
          <Button
            size="sm"
            variant="outline"
            type="button"
            disabled={pageOffset === 0 || isPoolPagePending}
            onClick={onPreviousPage}
          >
            上一页
          </Button>
          <div className="truncate text-center text-xs text-muted-foreground tabular-nums">
            {pageOffset + 1} - {pageOffset + pagedStocks.length}
          </div>
          <Button
            size="sm"
            variant="outline"
            type="button"
            disabled={!pageHasMore || isPoolPagePending}
            onClick={onNextPage}
          >
            下一页
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

function KeyDataPanel({
  analysis,
  analysisError,
  catalogOptions,
  isAnalysisPending,
  onUpdateWeightIndicator,
  weightIndicators,
}: {
  analysis: PreviewChartContextResponse | null
  analysisError: unknown
  catalogOptions: IndicatorCatalog[]
  isAnalysisPending: boolean
  onUpdateWeightIndicator: (
    indicatorId: string,
    patch: Partial<WeightIndicator>
  ) => void
  stock: PreviewStockRow | null
  weightIndicators: WeightIndicator[]
}) {
  const quote = analysis?.selected_quote ?? null

  return (
    <Card
      size="sm"
      className="min-h-[24rem] bg-transparent py-0 ring-0 xl:h-full"
    >
      <CardContent className="min-h-0 flex-1 overflow-y-auto py-3">
        <div className="flex flex-col gap-4">
          {isAnalysisPending ? (
            <Skeleton className="h-20 w-full" />
          ) : analysisError ? (
            <Alert variant="destructive">
              <AlertTitle>个股上下文加载失败</AlertTitle>
              <AlertDescription>
                {formatErrorMessage(analysisError)}
              </AlertDescription>
            </Alert>
          ) : null}

          <MetricSection meta={quote?.trade_date} title="行情">
            <DataRow label="开盘价" value={formatPrice(quote?.open_price)} />
            <DataRow label="最高价" value={formatPrice(quote?.high_price)} />
            <DataRow label="最低价" value={formatPrice(quote?.low_price)} />
            <DataRow label="收盘价" value={formatPrice(quote?.close_price)} />
            <DataRow
              label="前收盘价"
              value={formatPrice(quote?.prev_close_price)}
            />
            <DataRow
              label="涨跌幅"
              value={formatPercentPoint(quote?.pct_change)}
            />
            <DataRow
              label="振幅"
              value={formatPercentPoint(quote?.pct_amplitude)}
            />
            <DataRow
              label="成交量"
              value={formatCompactUnit(quote?.volume, 10000, "万手")}
            />
            <DataRow
              label="成交额"
              value={formatCompactUnit(quote?.amount, 100000000, "亿")}
            />
            <DataRow
              label="涨停价"
              value={formatPrice(quote?.limit_up_price)}
            />
            <DataRow
              label="跌停价"
              value={formatPrice(quote?.limit_down_price)}
            />
          </MetricSection>

          <MetricSection title="估值与财务">
            <DataRow
              label="总市值"
              value={formatCompactUnit(quote?.a_market_cap, 100000000, "亿")}
            />
            <DataRow label="PE(TTM)" value={formatNumber(quote?.pe_ttm)} />
            <DataRow label="ROE" value={formatRatio(quote?.roe)} />
          </MetricSection>

          <CompactWeightTuningSection
            catalogOptions={catalogOptions}
            weightIndicators={weightIndicators}
            onUpdateWeightIndicator={onUpdateWeightIndicator}
          />
        </div>
      </CardContent>
    </Card>
  )
}

function CompactWeightTuningSection({
  catalogOptions,
  onUpdateWeightIndicator,
  weightIndicators,
}: {
  catalogOptions: IndicatorCatalog[]
  onUpdateWeightIndicator: (
    indicatorId: string,
    patch: Partial<WeightIndicator>
  ) => void
  weightIndicators: WeightIndicator[]
}) {
  return (
    <MetricSection title="权重配置">
      <div className="flex flex-col gap-0.5">
        {weightIndicators.length === 0 ? (
          <div className="text-xs text-muted-foreground">暂无权重项</div>
        ) : (
          weightIndicators.map((indicator) => {
            const clampedScore = clampScore(indicator.score)

            return (
              <div
                key={indicator.id}
                className="grid grid-cols-[minmax(0,1.25fr)_minmax(4.5rem,0.9fr)_2.75rem] items-center gap-1.5 border-b border-border/30 py-1 last:border-b-0"
              >
                <div
                  className="truncate text-xs text-muted-foreground"
                  title={formatWeightIndicator(indicator, { catalogOptions })}
                >
                  {formatWeightIndicator(indicator, { catalogOptions })}
                </div>
                <WeightScoreSlider
                  className="[&_[data-slot=slider-range]]:bg-muted-foreground/35 [&_[data-slot=slider-thumb]]:size-2 [&_[data-slot=slider-thumb]]:border-muted-foreground/35 [&_[data-slot=slider-thumb]]:bg-background [&_[data-slot=slider-track]]:h-0.5 [&_[data-slot=slider-track]]:bg-muted/70"
                  value={clampedScore}
                  onValueChange={(nextValue) =>
                    onUpdateWeightIndicator(indicator.id, {
                      score: clampScore(nextValue),
                    })
                  }
                />
                <Input
                  className="h-6 px-1 text-center text-xs text-muted-foreground tabular-nums"
                  max={100}
                  min={0}
                  type="number"
                  value={String(indicator.score)}
                  onChange={(event) =>
                    onUpdateWeightIndicator(indicator.id, {
                      score: Number(event.target.value),
                    })
                  }
                />
              </div>
            )
          })
        )}
      </div>
    </MetricSection>
  )
}

function MetricSection({
  children,
  meta,
  title,
}: {
  children: ReactNode
  meta?: string | null
  title: string
}) {
  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2 text-[11px] font-medium text-muted-foreground">
        <span>{title}</span>
        {meta ? <span className="tabular-nums">{meta}</span> : null}
      </div>
      <div className="flex flex-col gap-1">{children}</div>
    </section>
  )
}

function DataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3 border-b border-border/50 py-1.5 last:border-b-0 last:pb-0">
      <div className="truncate text-xs text-muted-foreground">{label}</div>
      <div className="max-w-40 truncate text-xs font-medium tabular-nums">
        {value}
      </div>
    </div>
  )
}

function InlineValueRows({ rows }: { rows: PreviewValueRow[] }) {
  if (rows.length === 0) {
    return <span className="text-xs text-muted-foreground">-</span>
  }

  return (
    <div
      className="w-full truncate text-xs text-muted-foreground tabular-nums"
      title={formatValueRows(rows)}
    >
      {formatValueRows(rows)}
    </div>
  )
}

function formatScoreItems(scoreItems: { label: string; score: number }[]) {
  if (scoreItems.length === 0) {
    return "-"
  }

  return scoreItems.map((item) => item.label).join(" / ")
}

function formatValueRows(rows: PreviewValueRow[]) {
  if (rows.length === 0) {
    return "-"
  }

  return rows.map((row) => `${row.label} ${row.value}`).join(" / ")
}

function formatStockSubtitle(
  securityCode: string | null | undefined,
  boardLabel: string | null | undefined
) {
  const parts = [securityCode, boardLabel === "-" ? null : boardLabel].filter(
    Boolean
  )

  return parts.join(" / ") || "-"
}

function formatCompactDate(date: string) {
  const [, month, day] = date.split("-")

  return `${Number(month)}/${Number(day)}`
}

function formatPrice(value: number | null | undefined) {
  return typeof value === "number" ? priceFormatter.format(value) : "-"
}

function formatNumber(value: number | null | undefined) {
  return typeof value === "number" ? compactFormatter.format(value) : "-"
}

function formatRatio(value: number | null | undefined) {
  return typeof value === "number" ? percentFormatter.format(value) : "-"
}

function formatCompactUnit(
  value: number | null | undefined,
  divisor: number,
  unit: string
) {
  return typeof value === "number"
    ? `${compactFormatter.format(value / divisor)} ${unit}`
    : "-"
}

function formatHoverVolume(value: number | null | undefined) {
  return typeof value === "number"
    ? `${compactFormatter.format(value / 10000)}万`
    : "-"
}

function formatPercentPoint(value: number | null | undefined) {
  if (typeof value !== "number") {
    return "-"
  }

  const sign = value > 0 ? "+" : ""
  return `${sign}${value.toFixed(2)}%`
}

function formatErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message
  }

  return String(error || "Unknown error")
}

export { StockPoolPreviewWorkbench }
