export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue }

export type JsonRecord = Record<string, JsonValue>

export type MetricValueKind =
  | "numeric"
  | "integer"
  | "boolean"
  | "string"
  | "date"

export type Operator =
  | "eq"
  | "ne"
  | "lt"
  | "lte"
  | "gt"
  | "gte"
  | "between"
  | "is_null"
  | "crosses_above"
  | "crosses_below"

export type MetricCross = {
  previous_metric: string
}

export type MetricDisplay = {
  group?: string | null
  label_zh?: string | null
  unit?: string | null
  sort_order?: number | null
}

export type MetricDefinition = {
  logical_metric: string
  mart_database: string
  mart_table: string
  column_name: string
  value_kind: MetricValueKind
  allow_filter: boolean
  allow_scoring: boolean
  allowed_ops: Operator[]
  null_policy: "no_match" | "match" | "error"
  default_output: boolean
  description?: string | null
  cross?: MetricCross | null
  display?: MetricDisplay | null
}

export type UniverseSpec = {
  base: string
  exclude_st: boolean
  exclude_suspend: boolean
  include_security_codes: string[]
  exclude_security_codes: string[]
}

export type FilterExpr =
  | { type: "all"; conditions: FilterExpr[] }
  | { type: "any"; conditions: FilterExpr[] }
  | { type: "not"; condition: FilterExpr }
  | {
      type: "compare"
      left: Operand
      op: Operator
      right?: Operand | null
    }

export type Operand =
  | { type: "metric"; name: string }
  | { type: "number"; value: number }
  | { type: "bool"; value: boolean }
  | { type: "string"; value: string }
  | { type: "range"; min: Operand; max: Operand }
  | { type: "binary"; op: "multiply"; left: Operand; right: Operand }

export type ScoringSpec = {
  rules: ScoringRule[]
  clamp: ScoreClamp
}

export type ScoringRule =
  | {
      type: "conditional_points"
      name: string
      condition: FilterExpr
      points: number
    }
  | {
      type: "weighted_metric"
      name: string
      metric: string
      weight: number
    }

export type ScoreClamp = {
  min: number
  max: number
}

export type RuleVersionSpec = {
  universe: UniverseSpec
  pool_filters: FilterExpr
  scoring: ScoringSpec
  top_n_default: number
  output_metrics: string[]
}

export type ChunkPlanRecord = {
  chunk_no: number
  start_date: string
  end_date: string
}

export type ExplainResponse = {
  sql?: string
  sql_hash?: string
  compiled_sql_hash?: string
  required_metrics?: string[]
  required_marts?: string[]
  required_columns?: Record<string, string[]>
  chunk_plan?: ChunkPlanRecord[]
  [key: string]: JsonValue | string[] | ChunkPlanRecord[] | undefined
}

export type StrategyPreviewRequest = {
  rule: RuleVersionSpec
  start_date: string
  end_date: string
  preview_row_limit: number
  top_n?: number
}

export type StrategyPreviewSignal = {
  security_code: string
  security_name?: string | null
  exchange_code?: string | null
  security_board?: string | null
  raw_score: number
  score: number
  signal_rank: number
  is_buy_signal: boolean
  score_breakdown: JsonValue
  selected_metrics: JsonValue
  raw_values: JsonValue
}

export type StrategyPreviewTradeDate = {
  trade_date: string
  pool_count: number
  signals: StrategyPreviewSignal[]
}

export type StrategyPreviewResponse = {
  preview_id: string
  sql_hash: string
  required_metrics: string[]
  required_marts: string[]
  required_columns: Record<string, string[]>
  start_date: string
  end_date: string
  preview_row_limit: number
  top_n: number
  trade_dates: StrategyPreviewTradeDate[]
}

export type StrategyPreviewTimelineRequest = {
  rule: RuleVersionSpec
  start_date: string
  end_date: string
}

export type StrategyPreviewTimelineTradeDate = {
  trade_date: string
  pool_count: number
}

export type StrategyPreviewTimelineResponse = {
  preview_id: string
  sql_hash: string
  required_metrics: string[]
  required_marts: string[]
  required_columns: Record<string, string[]>
  start_date: string
  end_date: string
  trade_dates: StrategyPreviewTimelineTradeDate[]
}

export type StrategyPreviewPoolPageRequest = {
  rule: RuleVersionSpec
  trade_date: string
  limit: number
  offset: number
  sort?: "score_desc"
  security_code?: string
}

export type StrategyPreviewPoolPageResponse = {
  trade_date: string
  pool_count: number
  items: StrategyPreviewSignal[]
  limit: number
  offset: number
  has_more: boolean
}

export type Adjustment = "forward_adjusted" | "backward_adjusted" | "unadjusted"

export type ChartOhlc = {
  open: number
  high: number
  low: number
  close: number
}

export type ChartSeriesRow = {
  trade_date: string
  ohlc?: ChartOhlc | null
  volume?: number | null
  ma?: Record<string, number | null>
  price_overlays?: Record<string, number | null>
  kdj?: Record<string, number | null>
  rsi?: Record<string, number | null>
  macd?: Record<string, number | null>
  boll?: Record<string, number | null>
}

export type QuoteMartRow = {
  security_code: string
  trade_date: string
  open_price?: number | null
  high_price?: number | null
  low_price?: number | null
  close_price?: number | null
  prev_close_price?: number | null
  prev_close_price_unadj?: number | null
  volume?: number | null
  amount?: number | null
  pct_amplitude?: number | null
  pct_change?: number | null
  limit_up_price?: number | null
  limit_down_price?: number | null
  a_market_cap?: number | null
  pe_ttm?: number | null
  roe?: number | null
}

export type PreviewSecurityAnalysisRequest = {
  rule: RuleVersionSpec
  trade_date: string
  security_code: string
  adjustment?: Adjustment
  lookback_trading_days?: number
  ma_windows?: string
  include_quote_rows?: boolean
}

export type ChartMaMetadata = {
  requested_windows: number[]
  default_visible_windows: number[]
  available_windows: number[]
  adjustment: Adjustment
  basis_adjustment?: Adjustment
  status: "available" | string
}

export type ChartPriceOverlayMetadata = {
  default_visible_keys: string[]
  available_keys: string[]
  adjustment: Adjustment
  status: "available" | "forward_adjusted_only" | string
}

export type SecurityAnalysisResponse = {
  run_id?: string
  trade_date: string
  security_code: string
  security_name?: string | null
  exchange_code?: string | null
  security_board?: string | null
  source: "signals" | "pool" | "preview"
  adjustment: Adjustment
  result_snapshot: {
    rank?: number | null
    signal_rank?: number | null
    score?: number | null
    score_breakdown?: JsonValue
    selected_metrics: JsonValue
    raw_values?: JsonValue
    filter_snapshot?: JsonValue
  }
  chart_window: {
    start_date: string
    end_date: string
    lookback_trading_days: number
  }
  chart: {
    ma?: ChartMaMetadata
    price_overlays?: ChartPriceOverlayMetadata
    indicator_panels?: string[]
    series: ChartSeriesRow[]
  }
  quote_rows?: QuoteMartRow[]
  selected_quote?: QuoteMartRow | null
}

export type MetricsQuery = {
  mart_table?: string
  value_kind?: string
  allow_filter?: boolean
  allow_scoring?: boolean
  keyword?: string
}

export type ListResult<T> = {
  items: T[]
  limit: number
  offset: number
  has_more: boolean
  total?: number
}
