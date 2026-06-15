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
import type { ChartSeriesRow } from "@/types/rearview"

type SecurityAnalysisChartProps = {
  rows: ChartSeriesRow[]
  signalDate: string
  selectedDate: string
  visibleMaWindows: number[]
  maAvailableWindows: number[]
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
  up: "#15803d",
  down: "#b91c1c",
  volumeUp: "rgba(21, 128, 61, 0.28)",
  volumeDown: "rgba(185, 28, 28, 0.28)",
  signal: "#111827",
  selected: "#2563eb",
  ma5: "#2563eb",
  ma10: "#d97706",
  ma30: "#7c3aed",
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
  volume: Array<HistogramData<Time> | WhitespaceData<Time>>
  ma: Record<number, Array<LineData<Time> | WhitespaceData<Time>>>
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
  visibleMaWindows,
  maAvailableWindows,
  onSelectedDateChange,
}: SecurityAnalysisChartProps) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const candleSeriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null)
  const markersRef = useRef<ISeriesMarkersPluginApi<Time> | null>(null)

  const data = useMemo(() => buildChartData(rows), [rows])
  const availableMaSet = useMemo(
    () => new Set(maAvailableWindows),
    [maAvailableWindows],
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
    volumeSeries.setData(data.volume)

    for (const window of visibleMaWindows) {
      if (!availableMaSet.has(window)) {
        continue
      }
      const series = chart.addSeries(
        LineSeries,
        {
          color: maColor(colors, window),
          lastValueVisible: false,
          lineWidth: 1,
          priceLineVisible: false,
          title: `MA${window}`,
        },
        0,
      )
      series.setData(data.ma[window] ?? [])
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
    availableMaSet,
    data,
    onSelectedDateChange,
    rows.length,
    visibleMaWindows,
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
      <div className="grid gap-2 border-t px-3 py-2 text-xs text-muted-foreground sm:grid-cols-4">
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
    volume: rows.map((row) => {
      const value = finiteNumber(row.volume)
      if (value === null) {
        return { time: row.trade_date as Time }
      }
      return {
        color:
          row.ohlc && row.ohlc.close >= row.ohlc.open
            ? FALLBACK_COLORS.volumeUp
            : FALLBACK_COLORS.volumeDown,
        time: row.trade_date as Time,
        value,
      }
    }),
    ma: {
      5: lineData(rows, (row) => row.ma["5"]),
      10: lineData(rows, (row) => row.ma["10"]),
      30: lineData(rows, (row) => row.ma["30"]),
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

function maColor(colors: ChartColors, window: number) {
  if (window === 5) {
    return colors.ma5
  }
  if (window === 10) {
    return colors.ma10
  }
  return colors.ma30
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
