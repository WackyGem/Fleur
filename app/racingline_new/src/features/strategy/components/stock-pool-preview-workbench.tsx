import { useEffect, useMemo, useRef, useState, type ReactNode } from "react"
import { CandlestickSeries, createChart } from "lightweight-charts"

import { Badge } from "@/components/ui/badge"
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { WeightScoreSlider } from "@/features/strategy/components/weight-score-slider"
import type {
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import {
  clampScore,
  formatComparableIndicator,
  getScaledWeightIndicators,
} from "@/features/strategy/utils"
import type { JsonValue, StrategyPreviewResponse } from "@/types/rearview"

type CandlePoint = {
  close: number
  high: number
  low: number
  open: number
  time: string
  volume: number
}

type StockPoolItem = {
  candles: CandlePoint[]
  code: string
  marketCap: number
  name: string
  peTtm: number
  roe: number
}

type StockSnapshot = {
  amount: number
  close: number
  high: number
  limitDown: number
  limitUp: number
  low: number
  open: number
  pctAmplitude: number
  pctChange: number
  prevClose: number
  volume: number
}

type WeightScoreItem = {
  id: string
  label: string
  score: number
}

type DailyPoolStock = {
  code: string
  industry: string
  name: string
  rank: number
  score: number
  scoreItems: WeightScoreItem[]
}

type DailyStockPool = {
  averageScore: number
  date: string
  poolCount: number
  stocks: DailyPoolStock[]
}

type BadgeVariant =
  | "default"
  | "secondary"
  | "destructive"
  | "outline"
  | "ghost"
  | "link"

type StockPoolPreviewWorkbenchProps = {
  appliedWeightIndicators: WeightIndicator[]
  conditionGroups: StrategyConditionGroup[]
  draftWeightIndicators: WeightIndicator[]
  hasStrategyInput: boolean
  onDraftWeightScoreChange: (indicatorId: string, score: number) => void
  previewResult?: StrategyPreviewResponse | null
}

const adjustmentOptions = [
  { label: "除权", value: "none" },
  { label: "前复权", value: "forward" },
  { label: "后复权", value: "backward" },
] as const

const trendLineOptions = ["MA5", "MA10", "MA30", "MA60"] as const

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

const previewStock: StockPoolItem = {
  code: "600519.SH",
  name: "贵州茅台",
  marketCap: 19650,
  peTtm: 24.8,
  roe: 0.321,
  candles: buildCandles("2026-03-02", 1498, 0.08, 0.62),
}

const stockPoolCandidates = [
  { code: "600519.SH", industry: "白酒", name: "贵州茅台" },
  { code: "300750.SZ", industry: "电池", name: "宁德时代" },
  { code: "600036.SH", industry: "银行", name: "招商银行" },
  { code: "601318.SH", industry: "保险", name: "中国平安" },
  { code: "000858.SZ", industry: "白酒", name: "五粮液" },
  { code: "002594.SZ", industry: "汽车", name: "比亚迪" },
  { code: "600276.SH", industry: "医药", name: "恒瑞医药" },
  { code: "601899.SH", industry: "有色", name: "紫金矿业" },
  { code: "600900.SH", industry: "电力", name: "长江电力" },
  { code: "000333.SZ", industry: "家电", name: "美的集团" },
  { code: "688981.SH", industry: "半导体", name: "中芯国际" },
  { code: "601012.SH", industry: "光伏", name: "隆基绿能" },
] as const

function StockPoolPreviewWorkbench({
  appliedWeightIndicators,
  conditionGroups,
  draftWeightIndicators,
  hasStrategyInput,
  onDraftWeightScoreChange,
  previewResult,
}: StockPoolPreviewWorkbenchProps) {
  const selectedStock = previewStock
  const selectedSnapshot = getStockSnapshot(selectedStock)
  const dailyStockPools = useMemo(
    () => {
      if (previewResult) {
        return buildDailyStockPoolsFromPreview(
          previewResult,
          appliedWeightIndicators
        )
      }

      return buildDailyStockPools(
        selectedStock,
        conditionGroups,
        appliedWeightIndicators,
        hasStrategyInput || appliedWeightIndicators.length > 0
      )
    },
    [
      appliedWeightIndicators,
      conditionGroups,
      hasStrategyInput,
      previewResult,
      selectedStock,
    ]
  )
  const latestTradeDate = dailyStockPools.at(-1)?.date ?? ""
  const [selectedTradeDate, setSelectedTradeDate] = useState(latestTradeDate)
  const selectedDailyPool =
    dailyStockPools.find((pool) => pool.date === selectedTradeDate) ??
    dailyStockPools.at(-1)

  if (!selectedDailyPool) {
    return null
  }

  return (
    <div
      className="grid h-full min-h-[46rem] grid-cols-1 xl:min-h-0 xl:grid-cols-[minmax(34rem,1fr)_auto_20rem]"
      data-has-strategy-input={hasStrategyInput}
    >
      <div className="flex min-h-0 flex-col gap-3 pt-5">
        <KLinePanel stock={selectedStock} />
        <Separator />
        <DailyStockPoolPanel
          dailyStockPools={dailyStockPools}
          selectedDate={selectedDailyPool.date}
          selectedPool={selectedDailyPool}
          onSelectedDateChange={setSelectedTradeDate}
        />
      </div>
      <Separator className="my-3 xl:hidden" />
      <Separator orientation="vertical" className="hidden xl:block" />
      <div className="min-h-0 xl:h-full xl:pt-5">
        <KeyDataPanel
          draftWeightIndicators={draftWeightIndicators}
          snapshot={selectedSnapshot}
          stock={selectedStock}
          onDraftWeightScoreChange={onDraftWeightScoreChange}
        />
      </div>
    </div>
  )
}

function KLinePanel({ stock }: { stock: StockPoolItem }) {
  const [adjustmentMode, setAdjustmentMode] =
    useState<(typeof adjustmentOptions)[number]["value"]>("forward")
  const [trendLines, setTrendLines] = useState<string[]>(["MA5", "MA10"])
  const adjustmentLabel =
    adjustmentOptions.find((option) => option.value === adjustmentMode)
      ?.label ?? "前复权"

  return (
    <Card
      size="sm"
      className="min-h-[27rem] shrink-0 bg-transparent py-0 ring-0 sm:min-h-[28rem] xl:min-h-[29rem] xl:pr-4"
    >
      <CardHeader className="grid gap-2 px-0 pt-2 pb-1 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-start">
        <div className="min-h-[3.75rem] min-w-0">
          <CardTitle className="flex h-[3.75rem] min-w-0 flex-col justify-between group-data-[size=sm]/card:text-xl">
            <span className="flex h-7 items-center truncate leading-7">
              {stock.name}
            </span>
            <span className="flex h-7 items-center text-sm leading-5 font-normal text-muted-foreground tabular-nums">
              {stock.code}
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
                  setAdjustmentMode(
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
              value={trendLines}
              onValueChange={(nextTrendLines) => {
                setTrendLines(nextTrendLines)
              }}
              variant="outline"
              size="sm"
              spacing={0}
              className="min-w-0 flex-wrap justify-end"
            >
              {trendLineOptions.map((option) => (
                <ToggleGroupItem
                  key={option}
                  value={option}
                  className="text-muted-foreground/70 aria-pressed:text-foreground"
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
          <CandlestickChart stock={stock} />
        </div>
      </CardContent>
    </Card>
  )
}

function CandlestickChart({ stock }: { stock: StockPoolItem }) {
  const containerRef = useRef<HTMLDivElement | null>(null)

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
          bottom: 0.12,
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
      stock.candles.map(({ close, high, low, open, time }) => ({
        close,
        high,
        low,
        open,
        time,
      }))
    )
    chart.timeScale().fitContent()

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0]

      if (!entry) {
        return
      }

      chart.applyOptions({
        height: Math.max(entry.contentRect.height, 180),
        width: entry.contentRect.width,
      })
    })

    resizeObserver.observe(container)

    return () => {
      resizeObserver.disconnect()
      chart.remove()
    }
  }, [stock])

  return <div ref={containerRef} className="h-full min-h-[12rem] w-full" />
}

function DailyStockPoolPanel({
  dailyStockPools,
  onSelectedDateChange,
  selectedDate,
  selectedPool,
}: {
  dailyStockPools: DailyStockPool[]
  onSelectedDateChange: (date: string) => void
  selectedDate: string
  selectedPool: DailyStockPool
}) {
  return (
    <Card
      size="sm"
      className="min-h-0 flex-1 bg-transparent py-0 ring-0 xl:pr-4"
    >
      <CardContent className="flex h-full min-h-0 flex-col gap-2 px-0 pt-0 pb-0">
        <div className="h-[32px] shrink-0 [scrollbar-width:thin] overflow-x-auto overflow-y-hidden overscroll-x-contain pb-3 [&::-webkit-scrollbar]:h-[2px] [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent">
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
          <Table className="min-w-[44rem] table-fixed">
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead className="h-7 w-8 px-1 text-right">#</TableHead>
                <TableHead className="h-7 w-40 px-1">股票</TableHead>
                <TableHead className="h-7 px-1">得分项</TableHead>
                <TableHead className="h-7 w-16 px-1 text-right">得分</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {selectedPool.stocks.map((stock) => (
                <TableRow key={stock.code}>
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
                    <div
                      className="w-full truncate text-muted-foreground tabular-nums"
                      title={formatScoreItems(stock.scoreItems)}
                    >
                      {formatScoreItems(stock.scoreItems)}
                    </div>
                  </TableCell>
                  <TableCell className="px-1 py-1 text-right">
                    <Badge variant={getScoreBadgeVariant(stock.score)}>
                      {stock.score.toFixed(1)}
                    </Badge>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  )
}

function KeyDataPanel({
  draftWeightIndicators,
  onDraftWeightScoreChange,
  snapshot,
  stock,
}: {
  draftWeightIndicators: WeightIndicator[]
  onDraftWeightScoreChange: (indicatorId: string, score: number) => void
  snapshot: StockSnapshot
  stock: StockPoolItem
}) {
  return (
    <Card
      size="sm"
      className="min-h-[24rem] bg-transparent py-0 ring-0 xl:h-full"
    >
      <CardContent className="min-h-0 flex-1 overflow-y-auto py-3">
        <div className="flex flex-col gap-4">
          <MetricSection title="行情">
            <DataRow
              label="开盘价"
              value={priceFormatter.format(snapshot.open)}
            />
            <DataRow
              label="最高价"
              value={priceFormatter.format(snapshot.high)}
            />
            <DataRow
              label="最低价"
              value={priceFormatter.format(snapshot.low)}
            />
            <DataRow
              label="收盘价"
              value={priceFormatter.format(snapshot.close)}
            />
            <DataRow
              label="前收盘价"
              value={priceFormatter.format(snapshot.prevClose)}
            />
            <DataRow
              label="涨跌幅"
              value={formatPercentPoint(snapshot.pctChange)}
            />
            <DataRow
              label="振幅"
              value={formatPercentPoint(snapshot.pctAmplitude)}
            />
            <DataRow
              label="成交量"
              value={`${compactFormatter.format(snapshot.volume / 10000)} 万手`}
            />
            <DataRow
              label="成交额"
              value={`${compactFormatter.format(snapshot.amount / 100000000)} 亿`}
            />
            <DataRow
              label="涨停价"
              value={priceFormatter.format(snapshot.limitUp)}
            />
            <DataRow
              label="跌停价"
              value={priceFormatter.format(snapshot.limitDown)}
            />
          </MetricSection>

          <MetricSection title="估值与财务">
            <DataRow
              label="总市值"
              value={`${compactFormatter.format(stock.marketCap)} 亿`}
            />
            <DataRow label="PE(TTM)" value={stock.peTtm.toFixed(1)} />
            <DataRow label="ROE" value={percentFormatter.format(stock.roe)} />
          </MetricSection>

          <Separator />

          <WeightControlSection
            weightIndicators={draftWeightIndicators}
            onWeightScoreChange={onDraftWeightScoreChange}
          />
        </div>
      </CardContent>
    </Card>
  )
}

function MetricSection({
  children,
  title,
}: {
  children: ReactNode
  title: string
}) {
  return (
    <section className="flex flex-col gap-2">
      <div className="text-[11px] font-medium text-muted-foreground">
        {title}
      </div>
      <div className="flex flex-col gap-1">{children}</div>
    </section>
  )
}

function DataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3 border-b border-border/50 py-1.5 last:border-b-0 last:pb-0">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="max-w-40 truncate text-xs font-medium tabular-nums">
        {value}
      </div>
    </div>
  )
}

function WeightControlSection({
  onWeightScoreChange,
  weightIndicators,
}: {
  onWeightScoreChange: (indicatorId: string, score: number) => void
  weightIndicators: WeightIndicator[]
}) {
  return (
    <MetricSection title="指标权重">
      <FieldGroup className="gap-3">
        {weightIndicators.map((indicator) => {
          const score = clampScore(indicator.score)

          return (
            <Field key={indicator.id} className="gap-1.5">
              <div className="grid grid-cols-[minmax(0,1fr)_2.5rem] items-center gap-2">
                <FieldLabel className="truncate text-xs font-normal">
                  {formatComparableIndicator(indicator)}
                </FieldLabel>
                <div className="text-right text-xs font-medium tabular-nums">
                  {score}
                </div>
              </div>
              <WeightScoreSlider
                value={score}
                onValueChange={(nextScore) => {
                  onWeightScoreChange(indicator.id, nextScore)
                }}
              />
            </Field>
          )
        })}
      </FieldGroup>
    </MetricSection>
  )
}

function buildDailyStockPools(
  stock: StockPoolItem,
  conditionGroups: StrategyConditionGroup[],
  weightIndicators: WeightIndicator[],
  hasStrategyInput: boolean
) {
  const tradingDates = stock.candles.slice(-63).map((candle) => candle.time)
  const conditionCount = conditionGroups.reduce(
    (total, group) => total + group.conditions.length,
    0
  )
  const weightCount = weightIndicators.length
  const averageWeightScore =
    weightCount > 0
      ? weightIndicators.reduce(
          (total, indicator) => total + indicator.score,
          0
        ) / weightCount
      : 0
  const strategyBoost = hasStrategyInput
    ? Math.min(
        20,
        conditionCount * 2.4 + weightCount * 3 + averageWeightScore * 0.12
      )
    : 6

  return tradingDates.map((date, dayIndex) => {
    const dateSeed = getDateSeed(date)
    const scaledWeightIndicators =
      getScaledWeightIndicators(weightIndicators).indicators
    const poolSize = Math.min(
      stockPoolCandidates.length,
      4 + ((dateSeed + dayIndex + conditionCount + weightCount) % 5)
    )
    const stocks = stockPoolCandidates
      .map((candidate, candidateIndex) => {
        const matchedConditions =
          conditionCount === 0
            ? 0
            : Math.max(
                1,
                Math.min(
                  conditionCount,
                  1 + ((dateSeed + candidateIndex * 3) % conditionCount)
                )
              )
        const scoreItems = buildWeightScoreItems(
          scaledWeightIndicators,
          dateSeed,
          candidateIndex
        )
        const weightContribution = Math.min(
          40,
          scoreItems.reduce((total, item) => total + item.score, 0)
        )
        const priceMomentum = Math.sin((dayIndex + candidateIndex) * 0.72) * 8
        const dispersion = (((dateSeed + candidateIndex * 17) % 100) / 100) * 8
        const score = clampPreviewScore(
          48 +
            strategyBoost +
            matchedConditions * 4.5 +
            weightContribution * 0.38 +
            priceMomentum +
            dispersion
        )

        return {
          ...candidate,
          rank: 0,
          score,
          scoreItems,
        }
      })
      .sort((a, b) => b.score - a.score)
      .slice(0, poolSize)
      .map((candidate, index) => ({
        ...candidate,
        rank: index + 1,
      }))
    const averageScore =
      stocks.reduce((total, candidate) => total + candidate.score, 0) /
      stocks.length

    return {
      averageScore,
      date,
      poolCount: stocks.length,
      stocks,
    }
  })
}

function buildDailyStockPoolsFromPreview(
  previewResult: StrategyPreviewResponse,
  weightIndicators: WeightIndicator[]
): DailyStockPool[] {
  const labelByRuleName = new Map(
    weightIndicators.map((indicator, index) => [
      `weight:${indicator.id}:${index + 1}`,
      formatComparableIndicator(indicator),
    ])
  )

  return previewResult.trade_dates.map((tradeDate) => {
    const stocks = tradeDate.signals.map((signal) => ({
      code: signal.security_code,
      industry: "-",
      name: signal.security_code,
      rank: signal.signal_rank,
      score: signal.score,
      scoreItems: buildPreviewScoreItems(
        signal.score_breakdown,
        labelByRuleName
      ),
    }))
    const averageScore =
      stocks.length > 0
        ? stocks.reduce((total, candidate) => total + candidate.score, 0) /
          stocks.length
        : 0

    return {
      averageScore,
      date: tradeDate.trade_date,
      poolCount: tradeDate.pool_count,
      stocks,
    }
  })
}

function buildPreviewScoreItems(
  scoreBreakdown: JsonValue,
  labelByRuleName: Map<string, string>
): WeightScoreItem[] {
  if (!isJsonRecord(scoreBreakdown)) {
    return []
  }

  return Object.entries(scoreBreakdown)
    .map(([key, value]) => {
      if (typeof value !== "number") {
        return null
      }

      return {
        id: key,
        label: labelByRuleName.get(key) ?? key,
        score: value,
      }
    })
    .filter((item): item is WeightScoreItem => item !== null)
}

function isJsonRecord(value: JsonValue): value is Record<string, JsonValue> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function buildWeightScoreItems(
  weightIndicators: ReturnType<typeof getScaledWeightIndicators>["indicators"],
  dateSeed: number,
  candidateIndex: number
): WeightScoreItem[] {
  return weightIndicators.map((indicator, indicatorIndex) => {
    const factor =
      0.72 + ((dateSeed + candidateIndex * 11 + indicatorIndex * 17) % 28) / 100
    const score = Number((indicator.scaledScore * 0.38 * factor).toFixed(1))

    return {
      id: indicator.id,
      label: formatComparableIndicator(indicator),
      score,
    }
  })
}

function formatScoreItems(scoreItems: WeightScoreItem[]) {
  if (scoreItems.length === 0) {
    return "-"
  }

  return scoreItems
    .map((item) => `${item.label} ${item.score.toFixed(1)}`)
    .join(" / ")
}

function getDateSeed(date: string) {
  return Array.from(date).reduce((total, char) => total + char.charCodeAt(0), 0)
}

function clampPreviewScore(score: number) {
  return Math.min(99, Math.max(0, Number(score.toFixed(1))))
}

function getScoreBadgeVariant(score: number): BadgeVariant {
  if (score >= 85) {
    return "default"
  }

  if (score >= 70) {
    return "secondary"
  }

  return "outline"
}

function formatCompactDate(date: string) {
  const [, month, day] = date.split("-")

  return `${Number(month)}/${Number(day)}`
}

function buildCandles(
  startDate: string,
  startPrice: number,
  drift: number,
  amplitude: number
) {
  const candles: CandlePoint[] = []
  const date = new Date(`${startDate}T00:00:00.000Z`)
  let previousClose = startPrice

  for (let index = 0; candles.length < 72; index += 1) {
    const day = date.getUTCDay()

    if (day !== 0 && day !== 6) {
      const wave =
        Math.sin(candles.length * 0.43) * amplitude +
        Math.cos(candles.length * 0.17) * amplitude * 0.38 +
        drift
      const open = previousClose * (1 + Math.sin(index * 0.31) * 0.003)
      const close = Math.max(1, open * (1 + wave / 100))
      const high = Math.max(open, close) * (1 + 0.006 + (index % 5) * 0.001)
      const low = Math.min(open, close) * (1 - 0.006 - (index % 4) * 0.001)
      const volume =
        260000 + Math.round((Math.sin(index * 0.51) + 1.45) * 84000)

      candles.push({
        close: roundPrice(close),
        high: roundPrice(high),
        low: roundPrice(low),
        open: roundPrice(open),
        time: date.toISOString().slice(0, 10),
        volume,
      })
      previousClose = close
    }

    date.setUTCDate(date.getUTCDate() + 1)
  }

  return candles
}

function getStockSnapshot(stock: StockPoolItem): StockSnapshot {
  const last = stock.candles[stock.candles.length - 1]
  const previous = stock.candles[stock.candles.length - 2]
  const pctChange = ((last.close - previous.close) / previous.close) * 100

  return {
    amount: last.close * last.volume * 100,
    close: last.close,
    high: last.high,
    limitDown: roundPrice(previous.close * 0.9),
    limitUp: roundPrice(previous.close * 1.1),
    low: last.low,
    open: last.open,
    pctAmplitude: ((last.high - last.low) / previous.close) * 100,
    pctChange,
    prevClose: previous.close,
    volume: last.volume,
  }
}

function roundPrice(value: number) {
  return Number(value.toFixed(2))
}

function formatPercentPoint(value: number) {
  const sign = value > 0 ? "+" : ""
  return `${sign}${value.toFixed(2)}%`
}

export { StockPoolPreviewWorkbench }
