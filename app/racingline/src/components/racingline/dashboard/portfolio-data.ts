type MetricKind = "percent" | "ratio"

type Metric = {
  label: string
  value: number | null
  kind: MetricKind
  tone?: "up" | "down" | "neutral"
}

type CurvePoint = {
  time: string
  nav: number
  benchmark: number
}

type SignalStock = {
  code: string
  name: string
  score: number
  rank?: number
  signalDate?: string
  executionDate?: string
}

type PortfolioCardData = {
  id: string
  name: string
  liveStatus:
    | "pending_first_run"
    | "queued"
    | "running"
    | "succeeded"
    | "failed"
  startDate: string
  backtestDays: number
  simulationDays: number
  latestNav: number | null
  recentChange: number | null
  returns: Metric[]
  risk: Metric[]
  efficiency: Metric[]
  relative: Metric[]
  todaySignals: SignalStock[]
  curve: CurvePoint[]
}

function formatMetricValue(metric: Metric) {
  if (metric.value === null) {
    return "--"
  }

  if (metric.kind === "percent") {
    return `${(metric.value * 100).toFixed(2)}%`
  }

  return metric.value.toFixed(2)
}

function getMetricToneClassName(metric: Metric) {
  if (metric.value === null) {
    return "text-muted-foreground"
  }

  if (metric.tone === "up") {
    return "text-[color:var(--portfolio-up)]"
  }

  if (metric.tone === "down") {
    return "text-[color:var(--portfolio-down)]"
  }

  return "text-foreground"
}

function formatChangeValue(value: number | null) {
  if (value === null) {
    return "--"
  }

  const sign = value > 0 ? "+" : ""
  return `${sign}${(value * 100).toFixed(2)}%`
}

function getChangeToneClassName(value: number | null) {
  if (value === null) {
    return "text-muted-foreground"
  }

  if (value > 0) {
    return "text-[color:var(--portfolio-up)]"
  }

  if (value < 0) {
    return "text-[color:var(--portfolio-down)]"
  }

  return "text-foreground"
}

function getScoreBadgeVariant(score: number) {
  if (score >= 85) {
    return "default"
  }

  if (score >= 70) {
    return "secondary"
  }

  return "outline"
}

export type { CurvePoint, Metric, PortfolioCardData, SignalStock }
export {
  formatChangeValue,
  formatMetricValue,
  getChangeToneClassName,
  getMetricToneClassName,
  getScoreBadgeVariant,
}
