import type { PreviewSnapshot } from "@/features/strategy/preview"

export type PoolCountTrendPoint = {
  count: number
  label: string
}

export function buildPoolCountTrendData(
  previewSnapshot: PreviewSnapshot | null
): PoolCountTrendPoint[] {
  const rows =
    previewSnapshot?.timeline?.trade_dates ??
    previewSnapshot?.result.trade_dates.map((tradeDate) => ({
      trade_date: tradeDate.trade_date,
      pool_count: tradeDate.pool_count,
    })) ??
    []
  const latestTradeDate = rows.at(-1)?.trade_date

  if (!latestTradeDate) {
    return []
  }

  const cutoff = subtractUtcMonths(latestTradeDate, 3)
  const recentRows = rows.filter((row) => row.trade_date >= cutoff)

  return recentRows.map((row) => ({
    count: row.pool_count,
    label: formatMonthDay(row.trade_date),
  }))
}

function subtractUtcMonths(isoDate: string, months: number) {
  const date = new Date(`${isoDate}T00:00:00Z`)
  date.setUTCMonth(date.getUTCMonth() - months)
  return date.toISOString().slice(0, 10)
}

function formatMonthDay(isoDate: string) {
  const [, month, day] = isoDate.split("-")
  return `${Number(month)}/${Number(day)}`
}
