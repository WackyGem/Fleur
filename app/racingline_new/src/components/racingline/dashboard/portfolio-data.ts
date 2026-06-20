type MetricKind = "percent" | "ratio"

type Metric = {
  label: string
  value: number
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
}

type PortfolioCardData = {
  id: string
  name: string
  startDate: string
  backtestDays: number
  simulationDays: number
  latestNav: number
  recentChange: number
  returns: Metric[]
  risk: Metric[]
  efficiency: Metric[]
  relative: Metric[]
  todaySignals: SignalStock[]
  curve: CurvePoint[]
}

const portfolioCards: PortfolioCardData[] = [
  {
    id: "dividend-low-vol-rotation",
    name: "红利低波轮动",
    startDate: "2025-06-16",
    backtestDays: 240,
    simulationDays: 16,
    latestNav: 1.1842,
    recentChange: 0.0086,
    returns: [
      { label: "持仓收益", value: 0.1842, kind: "percent", tone: "up" },
      { label: "年化收益", value: 0.2196, kind: "percent", tone: "up" },
    ],
    risk: [
      { label: "最大回撤", value: -0.0824, kind: "percent", tone: "down" },
      { label: "年化波动率", value: 0.1375, kind: "percent", tone: "neutral" },
      { label: "下行波动率", value: 0.0941, kind: "percent", tone: "neutral" },
    ],
    efficiency: [
      { label: "Sharpe Ratio", value: 1.42, kind: "ratio" },
      { label: "Sortino Ratio", value: 1.91, kind: "ratio" },
      { label: "Calmar Ratio", value: 2.66, kind: "ratio" },
      { label: "Treynor Ratio", value: 0.23, kind: "ratio" },
    ],
    relative: [
      { label: "Alpha", value: 0.041, kind: "percent", tone: "neutral" },
      { label: "Beta", value: 0.78, kind: "ratio", tone: "neutral" },
      {
        label: "Information Ratio",
        value: 0.88,
        kind: "ratio",
        tone: "neutral",
      },
    ],
    todaySignals: [
      { code: "600036.SH", name: "招商银行", score: 91.4 },
      { code: "600900.SH", name: "长江电力", score: 88.2 },
      { code: "601318.SH", name: "中国平安", score: 84.7 },
      { code: "601988.SH", name: "中国银行", score: 78.5 },
      { code: "000333.SZ", name: "美的集团", score: 73.6 },
      { code: "600519.SH", name: "贵州茅台", score: 69.1 },
    ],
    curve: [
      { time: "2025-06-16", nav: 1.0, benchmark: 1.0 },
      { time: "2025-07-01", nav: 1.012, benchmark: 1.004 },
      { time: "2025-07-16", nav: 1.026, benchmark: 1.012 },
      { time: "2025-08-01", nav: 1.044, benchmark: 1.018 },
      { time: "2025-08-18", nav: 1.071, benchmark: 1.029 },
      { time: "2025-09-03", nav: 1.093, benchmark: 1.041 },
      { time: "2025-09-19", nav: 1.115, benchmark: 1.052 },
      { time: "2025-10-10", nav: 1.102, benchmark: 1.046 },
      { time: "2025-10-29", nav: 1.138, benchmark: 1.061 },
      { time: "2025-11-18", nav: 1.173, benchmark: 1.071 },
      { time: "2025-12-03", nav: 1.1842, benchmark: 1.082 },
    ],
  },
  {
    id: "growth-trend-enhanced",
    name: "景气趋势增强",
    startDate: "2025-05-08",
    backtestDays: 326,
    simulationDays: 28,
    latestNav: 1.2637,
    recentChange: -0.0064,
    returns: [
      { label: "持仓收益", value: 0.2637, kind: "percent", tone: "up" },
      { label: "年化收益", value: 0.2874, kind: "percent", tone: "up" },
    ],
    risk: [
      { label: "最大回撤", value: -0.1168, kind: "percent", tone: "down" },
      { label: "年化波动率", value: 0.1843, kind: "percent", tone: "neutral" },
      { label: "下行波动率", value: 0.1214, kind: "percent", tone: "neutral" },
    ],
    efficiency: [
      { label: "Sharpe Ratio", value: 1.37, kind: "ratio" },
      { label: "Sortino Ratio", value: 1.76, kind: "ratio" },
      { label: "Calmar Ratio", value: 2.46, kind: "ratio" },
      { label: "Treynor Ratio", value: 0.19, kind: "ratio" },
    ],
    relative: [
      { label: "Alpha", value: 0.058, kind: "percent", tone: "neutral" },
      { label: "Beta", value: 0.91, kind: "ratio", tone: "neutral" },
      {
        label: "Information Ratio",
        value: 0.94,
        kind: "ratio",
        tone: "neutral",
      },
    ],
    todaySignals: [
      { code: "688981.SH", name: "中芯国际", score: 93.1 },
      { code: "300750.SZ", name: "宁德时代", score: 89.6 },
      { code: "002475.SZ", name: "立讯精密", score: 82.3 },
    ],
    curve: [
      { time: "2025-05-08", nav: 1.0, benchmark: 1.0 },
      { time: "2025-05-26", nav: 1.018, benchmark: 0.996 },
      { time: "2025-06-11", nav: 1.056, benchmark: 1.014 },
      { time: "2025-06-27", nav: 1.088, benchmark: 1.022 },
      { time: "2025-07-15", nav: 1.146, benchmark: 1.047 },
      { time: "2025-08-04", nav: 1.193, benchmark: 1.061 },
      { time: "2025-08-25", nav: 1.244, benchmark: 1.074 },
      { time: "2025-09-12", nav: 1.221, benchmark: 1.068 },
      { time: "2025-10-08", nav: 1.249, benchmark: 1.082 },
      { time: "2025-10-29", nav: 1.2637, benchmark: 1.091 },
    ],
  },
]

function formatMetricValue(metric: Metric) {
  if (metric.kind === "percent") {
    return `${(metric.value * 100).toFixed(2)}%`
  }

  return metric.value.toFixed(2)
}

function getMetricToneClassName(metric: Metric) {
  if (metric.tone === "up") {
    return "text-[color:var(--portfolio-up)]"
  }

  if (metric.tone === "down") {
    return "text-[color:var(--portfolio-down)]"
  }

  return "text-foreground"
}

function formatChangeValue(value: number) {
  const sign = value > 0 ? "+" : ""
  return `${sign}${(value * 100).toFixed(2)}%`
}

function getChangeToneClassName(value: number) {
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
  portfolioCards,
  formatChangeValue,
  formatMetricValue,
  getChangeToneClassName,
  getMetricToneClassName,
  getScoreBadgeVariant,
}
