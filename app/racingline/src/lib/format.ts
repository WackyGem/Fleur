import { ApiError } from "@/api/client"
import type {
  BuySignalRecord,
  JsonValue,
  PoolMemberRecord,
  RunDayRecord,
} from "@/types/rearview"

export function shortId(value?: string | null, size = 8) {
  if (!value) {
    return "-"
  }
  return value.length > size ? value.slice(0, size) : value
}

export function formatCount(value?: number | null) {
  return value === undefined || value === null ? "-" : value.toLocaleString()
}

export function formatScore(value?: number | null) {
  if (value === undefined || value === null || Number.isNaN(value)) {
    return "-"
  }
  return value.toLocaleString(undefined, {
    maximumFractionDigits: 4,
    minimumFractionDigits: 0,
  })
}

export function formatMoney(value?: number | null) {
  if (value === undefined || value === null || Number.isNaN(value)) {
    return "-"
  }
  return value.toLocaleString(undefined, {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  })
}

export function formatPct(value?: number | null) {
  if (value === undefined || value === null || Number.isNaN(value)) {
    return "-"
  }
  return (
    (value * 100).toLocaleString(undefined, {
      maximumFractionDigits: 2,
      minimumFractionDigits: 2,
    }) + "%"
  )
}

export function describeError(error: unknown) {
  if (error instanceof ApiError) {
    return [
      error.errorType ? `${error.errorType}: ${error.message}` : error.message,
      error.fieldPath ? `field_path=${error.fieldPath}` : "",
    ]
      .filter(Boolean)
      .join(" ")
  }

  if (error instanceof Error) {
    return error.message
  }

  return "unknown error"
}

export function jsonPreview(value: JsonValue | Record<string, JsonValue>) {
  return JSON.stringify(value, null, 2)
}

export function jsonEntries(value?: Record<string, JsonValue> | null) {
  return Object.entries(value ?? {}).sort(([left], [right]) =>
    left.localeCompare(right)
  )
}

export function metricColumns(
  rows: Array<BuySignalRecord | PoolMemberRecord>,
  preferred: string[] = []
) {
  const seen = new Set<string>()
  const columns: string[] = []

  for (const name of preferred) {
    if (name && !seen.has(name)) {
      seen.add(name)
      columns.push(name)
    }
  }

  for (const row of rows) {
    for (const name of Object.keys(row.selected_metrics ?? {})) {
      if (!seen.has(name)) {
        seen.add(name)
        columns.push(name)
      }
    }
  }

  return columns.slice(0, 8)
}

export function displayJsonValue(value: JsonValue | undefined) {
  if (value === undefined || value === null) {
    return "-"
  }
  if (typeof value === "number") {
    return formatScore(value)
  }
  if (typeof value === "boolean") {
    return value ? "true" : "false"
  }
  if (typeof value === "string") {
    return value
  }
  return JSON.stringify(value)
}

export function selectDefaultTradeDate(days: RunDayRecord[]) {
  const successfulDays = days
    .filter((day) => day.status === "succeeded")
    .sort((left, right) => right.trade_date.localeCompare(left.trade_date))

  return (
    successfulDays.find((day) => (day.signal_count ?? 0) > 0)?.trade_date ??
    successfulDays[0]?.trade_date ??
    days
      .slice()
      .sort((left, right) => right.trade_date.localeCompare(left.trade_date))[0]
      ?.trade_date ??
    ""
  )
}

export function compactList(values: string[]) {
  return values.filter(Boolean).join(", ") || "-"
}
