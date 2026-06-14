import { useEffect, useMemo, useRef } from "react"
import { createChart, LineSeries } from "lightweight-charts"
import type { IChartApi, ISeriesApi, LineData, Time } from "lightweight-charts"

import { MissingBackendState } from "@/components/racingline/data-state"
import type { RunDayRecord } from "@/types/rearview"

type RunProgressChartProps = {
  days: RunDayRecord[]
}

const chartPalette = {
  background: "#ffffff",
  grid: "#e5e5e5",
  line: "#171717",
  text: "#737373",
}

export function RunProgressChart({ days }: RunProgressChartProps) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const seriesRef = useRef<ISeriesApi<"Line"> | null>(null)

  const data = useMemo<LineData<Time>[]>(
    () =>
      days
        .filter((day) => day.status === "succeeded")
        .map((day) => ({
          time: day.trade_date,
          value: day.signal_count ?? 0,
        })),
    [days],
  )

  useEffect(() => {
    const container = containerRef.current
    if (!container || chartRef.current) {
      return
    }

    const chart = createChart(container, {
      autoSize: true,
      height: 180,
      layout: {
        attributionLogo: false,
        background: { color: chartPalette.background },
        textColor: chartPalette.text,
      },
      grid: {
        horzLines: { color: chartPalette.grid },
        vertLines: { color: chartPalette.grid },
      },
      rightPriceScale: {
        borderVisible: false,
      },
      timeScale: {
        borderVisible: false,
      },
    })
    const series = chart.addSeries(LineSeries, {
      color: chartPalette.line,
      lineWidth: 2,
    })

    chartRef.current = chart
    seriesRef.current = series

    return () => {
      chart.remove()
      chartRef.current = null
      seriesRef.current = null
    }
  }, [])

  useEffect(() => {
    seriesRef.current?.setData(data)
    chartRef.current?.timeScale().fitContent()
  }, [data])

  if (data.length === 0) {
    return (
      <MissingBackendState
        description="No successful day has signal_count data for the chart."
        title="No chart data"
      />
    )
  }

  return <div ref={containerRef} className="h-45 min-w-0" />
}
