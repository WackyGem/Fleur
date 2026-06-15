import { useEffect, useMemo, useRef } from "react"
import {
  CandlestickSeries,
  ColorType,
  createChart,
  createSeriesMarkers,
  CrosshairMode,
  HistogramSeries,
  LineSeries,
} from "lightweight-charts"
import type {
  CandlestickData,
  HistogramData,
  IChartApi,
  ISeriesMarkersPluginApi,
  ISeriesApi,
  LineData,
  MouseEventParams,
  SeriesMarker,
  Time,
  WhitespaceData,
} from "lightweight-charts"

import { MissingBackendState } from "@/components/racingline/data-state"
import type { PriceOverlayKey } from "@/features/analysis/security-analysis"
import type { ChartSeriesRow } from "@/types/rearview"

const PRICE_OVERLAY_CONFIG: Array<{
  key: PriceOverlayKey
  title: string
  legend: string
}> = [
  { key: "price_ma_5", title: "MA5", legend: "MA5" },
  { key: "price_ma_10", title: "MA10", legend: "MA10" },
  { key: "price_ma_30", title: "MA30", legend: "MA30" },
  { key: "price_ema2_10", title: "EMA2-10", legend: "EMA2-10" },
  {
    key: "price_avg_ma_3_6_12_24",
    title: "AVG 3/6/12/24",
    legend: "AVG 3/6/12/24",
  },
  {
    key: "price_avg_ma_14_28_57_114",
    title: "AVG 14/28/57/114",
    legend: "AVG 14/28/57/114",
  },
]

type SecurityAnalysisChartProps = {
  rows: ChartSeriesRow[]
  signalDate: string
  selectedDate: string
  availablePriceOverlays: PriceOverlayKey[]
  visiblePriceOverlays: PriceOverlayKey[]
  onSelectedDateChange: (tradeDate: string) => void
}

type ChartColors = {
  background: string
  text: string
  grid: string
  up: string
  down: string
  volumeUp: string
  volumeDown: string
  signal: string
  selected: string
  ma5: string
  ma10: string
  ma30: string
  ema2_10: string
  avgMaShort: string
  avgMaLong: string
  k: string
  d: string
  j: string
  rsi6: string
  rsi12: string
  rsi24: string
  macdDif: string
  macdDea: string
  macdHistogramUp: string
  macdHistogramDown: string
  bollMid: string
  bollUp: string
  bollDown: string
}

const FALLBACK_COLORS: ChartColors = {
  background: "#ffffff",
  text: "#525252",
  grid: "#e5e5e5",
  up: "#dc2626",
  down: "#16a34a",
  volumeUp: "rgba(220, 38, 38, 0.28)",
  volumeDown: "rgba(22, 163, 74, 0.28)",
  signal: "#111827",
  selected: "#2563eb",
  ma5: "#2563eb",
  ma10: "#d97706",
  ma30: "#7c3aed",
  ema2_10: "#0891b2",
  avgMaShort: "#a16207",
  avgMaLong: "#be123c",
  k: "#2563eb",
  d: "#d97706",
  j: "#7c3aed",
  rsi6: "#0891b2",
  rsi12: "#4f46e5",
  rsi24: "#a16207",
  macdDif: "#2563eb",
  macdDea: "#d97706",
  macdHistogramUp: "rgba(21, 128, 61, 0.55)",
  macdHistogramDown: "rgba(185, 28, 28, 0.55)",
  bollMid: "#525252",
  bollUp: "#2563eb",
  bollDown: "#7c3aed",
}

type ChartData = {
  candles: Array<CandlestickData<Time> | WhitespaceData<Time>>
  priceOverlays: Record<
    PriceOverlayKey,
    Array<LineData<Time> | WhitespaceData<Time>>
  >
  kdjK: Array<LineData<Time> | WhitespaceData<Time>>
  kdjD: Array<LineData<Time> | WhitespaceData<Time>>
  kdjJ: Array<LineData<Time> | WhitespaceData<Time>>
  rsi6: Array<LineData<Time> | WhitespaceData<Time>>
  rsi12: Array<LineData<Time> | WhitespaceData<Time>>
  rsi24: Array<LineData<Time> | WhitespaceData<Time>>
  macdDif: Array<LineData<Time> | WhitespaceData<Time>>
  macdDea: Array<LineData<Time> | WhitespaceData<Time>>
  macdHistogram: Array<HistogramData<Time> | WhitespaceData<Time>>
  bollMid: Array<LineData<Time> | WhitespaceData<Time>>
  bollUp: Array<LineData<Time> | WhitespaceData<Time>>
  bollDown: Array<LineData<Time> | WhitespaceData<Time>>
  candleCloseByDate: Map<string, number>
  dates: Set<string>
}

export function SecurityAnalysisChart({
  rows,
  signalDate,
  selectedDate,
  availablePriceOverlays,
  visiblePriceOverlays,
  onSelectedDateChange,
}: SecurityAnalysisChartProps) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const candleSeriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null)
  const markersRef = useRef<ISeriesMarkersPluginApi<Time> | null>(null)

  const data = useMemo(() => buildChartData(rows), [rows])
  const availableOverlaySet = useMemo(
    () => new Set(availablePriceOverlays),
    [availablePriceOverlays],
  )

  useEffect(() => {
    const container = containerRef.current
    if (!container || rows.length === 0) {
      return
    }

    const colors = readChartColors(container)
    const chart = createChart(container, {
      autoSize: true,
      height: 760,
      layout: {
        attributionLogo: false,
        background: { type: ColorType.Solid, color: colors.background },
        fontSize: 11,
        textColor: colors.text,
        panes: {
          enableResize: true,
          separatorColor: colors.grid,
          separatorHoverColor: colors.text,
        },
      },
      crosshair: {
        mode: CrosshairMode.Normal,
      },
      grid: {
        horzLines: { color: colors.grid },
        vertLines: { color: colors.grid },
      },
      rightPriceScale: {
        borderVisible: false,
      },
      timeScale: {
        borderVisible: false,
        rightOffset: 2,
        timeVisible: true,
      },
    })

    chart.addPane()
    chart.addPane()
    chart.addPane()
    chart.addPane()

    const candleSeries = chart.addSeries(
      CandlestickSeries,
      {
        borderVisible: false,
        downColor: colors.down,
        wickDownColor: colors.down,
        wickUpColor: colors.up,
        upColor: colors.up,
      },
      0,
    )
    candleSeries.setData(data.candles)

    const volumeSeries = chart.addSeries(
      HistogramSeries,
      {
        priceFormat: { type: "volume" },
        priceScaleId: "volume",
      },
      0,
    )
    volumeSeries.priceScale().applyOptions({
      scaleMargins: { top: 0.82, bottom: 0 },
    })
    volumeSeries.setData(volumeData(rows, colors))

    for (const key of visiblePriceOverlays) {
      if (!availableOverlaySet.has(key)) {
        continue
      }
      const config = PRICE_OVERLAY_CONFIG.find((overlay) => overlay.key === key)
      addLine(
        chart,
        data.priceOverlays[key] ?? [],
        priceOverlayColor(colors, key),
        config?.title ?? key,
        0,
      )
    }

    addLine(chart, data.kdjK, colors.k, "K", 1)
    addLine(chart, data.kdjD, colors.d, "D", 1)
    addLine(chart, data.kdjJ, colors.j, "J", 1)
    addLine(chart, data.rsi6, colors.rsi6, "RSI6", 2)
    addLine(chart, data.rsi12, colors.rsi12, "RSI12", 2)
    addLine(chart, data.rsi24, colors.rsi24, "RSI24", 2)
    addLine(chart, data.macdDif, colors.macdDif, "DIF", 3)
    addLine(chart, data.macdDea, colors.macdDea, "DEA", 3)
    const macdHistogram = chart.addSeries(
      HistogramSeries,
      {
        priceFormat: { type: "price", precision: 4, minMove: 0.0001 },
        title: "MACD",
      },
      3,
    )
    macdHistogram.setData(data.macdHistogram)
    addLine(chart, data.bollMid, colors.bollMid, "BOLL mid", 4)
    addLine(chart, data.bollUp, colors.bollUp, "BOLL up", 4)
    addLine(chart, data.bollDown, colors.bollDown, "BOLL down", 4)

    const panes = chart.panes()
    panes[0]?.setStretchFactor(7)
    panes[1]?.setStretchFactor(1)
    panes[2]?.setStretchFactor(1)
    panes[3]?.setStretchFactor(1)
    panes[4]?.setStretchFactor(1)

    const handleClick = (event: MouseEventParams<Time>) => {
      const tradeDate = timeToTradeDate(event.time)
      if (tradeDate && data.dates.has(tradeDate)) {
        onSelectedDateChange(tradeDate)
      }
    }
    chart.subscribeClick(handleClick)
    chart.timeScale().fitContent()

    chartRef.current = chart
    candleSeriesRef.current = candleSeries
    markersRef.current = createSeriesMarkers(candleSeries, [])

    return () => {
      chart.unsubscribeClick(handleClick)
      chart.remove()
      chartRef.current = null
      candleSeriesRef.current = null
      markersRef.current = null
    }
  }, [
    availableOverlaySet,
    data,
    onSelectedDateChange,
    rows.length,
    rows,
    visiblePriceOverlays,
  ])

  useEffect(() => {
    const markerApi = markersRef.current
    if (!markerApi) {
      return
    }

    const colors = containerRef.current
      ? readChartColors(containerRef.current)
      : FALLBACK_COLORS
    markerApi.setMarkers(markersForDates(signalDate, selectedDate, colors))
  }, [selectedDate, signalDate])

  useEffect(() => {
    const chart = chartRef.current
    const candleSeries = candleSeriesRef.current
    if (!chart || !candleSeries) {
      return
    }

    const selectedClose = data.candleCloseByDate.get(selectedDate)
    if (selectedClose === undefined) {
      chart.clearCrosshairPosition()
      return
    }

    chart.setCrosshairPosition(selectedClose, selectedDate as Time, candleSeries)
  }, [data.candleCloseByDate, selectedDate])

  if (rows.length === 0) {
    return (
      <div className="flex min-h-120 items-center justify-center rounded-md border bg-background">
        <MissingBackendState
          description="Rearview returned no mart quote rows for this security and window."
          title="No chart data"
        />
      </div>
    )
  }

  return (
    <div className="min-w-0 rounded-md border bg-background">
      <div ref={containerRef} className="h-190 min-w-0" />
      <div className="grid gap-2 border-t px-3 py-2 text-xs text-muted-foreground sm:grid-cols-5">
        <PanelLegend
          title="Price overlays"
          values={PRICE_OVERLAY_CONFIG.map((overlay) => overlay.legend)}
        />
        <PanelLegend title="KDJ" values={["K", "D", "J"]} />
        <PanelLegend title="RSI" values={["6", "12", "24"]} />
        <PanelLegend title="MACD" values={["DIF", "DEA", "histogram"]} />
        <PanelLegend title="BOLL" values={["mid", "up", "down"]} />
      </div>
    </div>
  )
}

function PanelLegend({ title, values }: { title: string; values: string[] }) {
  return (
    <div className="min-w-0">
      <div className="font-medium text-foreground">{title}</div>
      <div className="truncate">{values.join(" / ")}</div>
    </div>
  )
}

function buildChartData(rows: ChartSeriesRow[]): ChartData {
  const candleCloseByDate = new Map<string, number>()
  const dates = new Set(rows.map((row) => row.trade_date))

  return {
    candles: rows.map((row) => {
      const time = row.trade_date as Time
      if (!row.ohlc) {
        return { time }
      }
      candleCloseByDate.set(row.trade_date, row.ohlc.close)
      return {
        close: row.ohlc.close,
        high: row.ohlc.high,
        low: row.ohlc.low,
        open: row.ohlc.open,
        time,
      }
    }),
    priceOverlays: {
      price_ma_5: lineData(rows, (row) => priceOverlayValue(row, "price_ma_5")),
      price_ma_10: lineData(rows, (row) =>
        priceOverlayValue(row, "price_ma_10"),
      ),
      price_ma_30: lineData(rows, (row) =>
        priceOverlayValue(row, "price_ma_30"),
      ),
      price_ema2_10: lineData(rows, (row) =>
        priceOverlayValue(row, "price_ema2_10"),
      ),
      price_avg_ma_3_6_12_24: lineData(rows, (row) =>
        priceOverlayValue(row, "price_avg_ma_3_6_12_24"),
      ),
      price_avg_ma_14_28_57_114: lineData(rows, (row) =>
        priceOverlayValue(row, "price_avg_ma_14_28_57_114"),
      ),
    },
    kdjK: lineData(rows, (row) => row.kdj.k),
    kdjD: lineData(rows, (row) => row.kdj.d),
    kdjJ: lineData(rows, (row) => row.kdj.j),
    rsi6: lineData(rows, (row) => row.rsi["6"]),
    rsi12: lineData(rows, (row) => row.rsi["12"]),
    rsi24: lineData(rows, (row) => row.rsi["24"]),
    macdDif: lineData(rows, (row) => row.macd.dif),
    macdDea: lineData(rows, (row) => row.macd.dea),
    macdHistogram: histogramData(rows, (row) => row.macd.histogram),
    bollMid: lineData(rows, (row) => row.boll.mid_20_2),
    bollUp: lineData(rows, (row) => row.boll.up_20_2),
    bollDown: lineData(rows, (row) => row.boll.dn_20_2),
    candleCloseByDate,
    dates,
  }
}

function priceOverlayValue(row: ChartSeriesRow, key: PriceOverlayKey) {
  const overlayValue = row.price_overlays?.[key]
  if (overlayValue !== undefined) {
    return overlayValue
  }
  if (key === "price_ma_5") {
    return row.ma["5"]
  }
  if (key === "price_ma_10") {
    return row.ma["10"]
  }
  if (key === "price_ma_30") {
    return row.ma["30"]
  }
  return null
}

function volumeData(
  rows: ChartSeriesRow[],
  colors: ChartColors,
): Array<HistogramData<Time> | WhitespaceData<Time>> {
  return rows.map((row) => {
    const value = finiteNumber(row.volume)
    if (value === null) {
      return { time: row.trade_date as Time }
    }
    return {
      color:
        row.ohlc && row.ohlc.close >= row.ohlc.open
          ? colors.volumeUp
          : colors.volumeDown,
      time: row.trade_date as Time,
      value,
    }
  })
}

function lineData(
  rows: ChartSeriesRow[],
  selectValue: (row: ChartSeriesRow) => number | null | undefined,
): Array<LineData<Time> | WhitespaceData<Time>> {
  return rows.map((row) => {
    const value = finiteNumber(selectValue(row))
    return value === null
      ? { time: row.trade_date as Time }
      : { time: row.trade_date as Time, value }
  })
}

function histogramData(
  rows: ChartSeriesRow[],
  selectValue: (row: ChartSeriesRow) => number | null | undefined,
): Array<HistogramData<Time> | WhitespaceData<Time>> {
  return rows.map((row) => {
    const value = finiteNumber(selectValue(row))
    if (value === null) {
      return { time: row.trade_date as Time }
    }
    return {
      color:
        value >= 0
          ? FALLBACK_COLORS.macdHistogramUp
          : FALLBACK_COLORS.macdHistogramDown,
      time: row.trade_date as Time,
      value,
    }
  })
}

function addLine(
  chart: IChartApi,
  rows: Array<LineData<Time> | WhitespaceData<Time>>,
  color: string,
  title: string,
  paneIndex: number,
) {
  const series = chart.addSeries(
    LineSeries,
    {
      color,
      lastValueVisible: false,
      lineWidth: 1,
      priceLineVisible: false,
      title,
    },
    paneIndex,
  )
  series.setData(rows)
}

function markersForDates(
  signalDate: string,
  selectedDate: string,
  colors: ChartColors,
): SeriesMarker<Time>[] {
  if (signalDate === selectedDate) {
    return [
      {
        color: colors.signal,
        position: "belowBar",
        shape: "arrowUp",
        text: "Signal / selected day",
        time: signalDate as Time,
      },
    ]
  }

  return [
    {
      color: colors.signal,
      position: "belowBar",
      shape: "arrowUp",
      text: "Signal day",
      time: signalDate as Time,
    },
    {
      color: colors.selected,
      position: "inBar",
      shape: "circle",
      text: "Selected day",
      time: selectedDate as Time,
    },
  ]
}

function finiteNumber(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value) ? value : null
}

function readChartColors(element: HTMLElement): ChartColors {
  const styles = getComputedStyle(element)
  const read = (name: string, fallback: string) =>
    styles.getPropertyValue(name).trim() || fallback

  return {
    background: read("--background", FALLBACK_COLORS.background),
    text: read("--racingline-chart-muted", FALLBACK_COLORS.text),
    grid: read("--racingline-chart-grid", FALLBACK_COLORS.grid),
    up: read("--racingline-chart-up", FALLBACK_COLORS.up),
    down: read("--racingline-chart-down", FALLBACK_COLORS.down),
    volumeUp: read("--racingline-chart-volume-up", FALLBACK_COLORS.volumeUp),
    volumeDown: read(
      "--racingline-chart-volume-down",
      FALLBACK_COLORS.volumeDown,
    ),
    signal: read("--racingline-chart-signal", FALLBACK_COLORS.signal),
    selected: read("--racingline-chart-selected", FALLBACK_COLORS.selected),
    ma5: read("--racingline-chart-ma-5", FALLBACK_COLORS.ma5),
    ma10: read("--racingline-chart-ma-10", FALLBACK_COLORS.ma10),
    ma30: read("--racingline-chart-ma-30", FALLBACK_COLORS.ma30),
    ema2_10: read("--racingline-chart-ema2-10", FALLBACK_COLORS.ema2_10),
    avgMaShort: read(
      "--racingline-chart-avg-ma-short",
      FALLBACK_COLORS.avgMaShort,
    ),
    avgMaLong: read(
      "--racingline-chart-avg-ma-long",
      FALLBACK_COLORS.avgMaLong,
    ),
    k: read("--racingline-chart-k", FALLBACK_COLORS.k),
    d: read("--racingline-chart-d", FALLBACK_COLORS.d),
    j: read("--racingline-chart-j", FALLBACK_COLORS.j),
    rsi6: read("--racingline-chart-rsi-6", FALLBACK_COLORS.rsi6),
    rsi12: read("--racingline-chart-rsi-12", FALLBACK_COLORS.rsi12),
    rsi24: read("--racingline-chart-rsi-24", FALLBACK_COLORS.rsi24),
    macdDif: read("--racingline-chart-macd-dif", FALLBACK_COLORS.macdDif),
    macdDea: read("--racingline-chart-macd-dea", FALLBACK_COLORS.macdDea),
    macdHistogramUp: read(
      "--racingline-chart-macd-up",
      FALLBACK_COLORS.macdHistogramUp,
    ),
    macdHistogramDown: read(
      "--racingline-chart-macd-down",
      FALLBACK_COLORS.macdHistogramDown,
    ),
    bollMid: read("--racingline-chart-boll-mid", FALLBACK_COLORS.bollMid),
    bollUp: read("--racingline-chart-boll-up", FALLBACK_COLORS.bollUp),
    bollDown: read("--racingline-chart-boll-down", FALLBACK_COLORS.bollDown),
  }
}

function priceOverlayColor(colors: ChartColors, key: PriceOverlayKey) {
  if (key === "price_ma_5") {
    return colors.ma5
  }
  if (key === "price_ma_10") {
    return colors.ma10
  }
  if (key === "price_ma_30") {
    return colors.ma30
  }
  if (key === "price_ema2_10") {
    return colors.ema2_10
  }
  if (key === "price_avg_ma_3_6_12_24") {
    return colors.avgMaShort
  }
  return colors.avgMaLong
}

function timeToTradeDate(time: Time | undefined) {
  if (typeof time === "string") {
    return time
  }
  if (typeof time === "object") {
    const month = String(time.month).padStart(2, "0")
    const day = String(time.day).padStart(2, "0")
    return `${time.year}-${month}-${day}`
  }
  return null
}
