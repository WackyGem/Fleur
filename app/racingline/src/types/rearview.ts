export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue }

export type HealthResponse = {
  status: string
}

export type RuleSetRecord = {
  rule_set_id: string
  name: string
  description?: string | null
  owner?: string | null
  status: string
  tags: JsonValue
  current_version_id?: string | null
}

export type RuleVersionRecord = {
  rule_version_id: string
  rule_set_id: string
  version_no: number
  status: string
  top_n_default: number
  rule_hash: string
}

export type RunSummary = {
  day_count?: number
  pool_count?: number
  signal_count?: number
  [key: string]: JsonValue | undefined
}

export type RunRecord = {
  run_id: string
  rule_version_id: string
  rule_set_id?: string | null
  rule_set_name?: string | null
  rule_hash: string
  start_date: string
  end_date: string
  top_n: number
  status: string
  compiled_sql_hash?: string | null
  summary: RunSummary
  error_type?: string | null
  error_message?: string | null
}

export type FeeProfile = {
  commission_rate: number
  commission_rate_max: number
  min_commission: number
  stamp_duty_rate_sell: number
  transfer_fee_rate: number
  [key: string]: JsonValue
}

export type SlippageProfile = {
  mode?: string
  buy_bps: number
  sell_bps: number
  [key: string]: JsonValue | undefined
}

export type MarketFeeTemplateRecord = {
  market_fee_template_id: string
  market: string
  name: string
  currency: string
  fee_profile: FeeProfile
  slippage_profile: SlippageProfile
  is_default: boolean
  status: string
}

export type AccountTemplateRecord = {
  account_template_id: string
  rule_set_id: string
  market_fee_template_id?: string | null
  name: string
  initial_cash: number
  currency: string
  fee_profile: FeeProfile
  slippage_profile: SlippageProfile
  rebalance_policy: JsonRecord
  risk_exit_policy: JsonRecord
  is_default: boolean
  status: string
}

export type PortfolioSummary = {
  initial_cash?: number
  ending_equity?: number
  total_return?: number
  max_drawdown?: number
  trade_count?: number
  total_fee?: number
  warning_count?: number
  [key: string]: JsonValue | undefined
}

export type PortfolioRunRecord = {
  portfolio_run_id: string
  source_run_id: string
  rule_version_id: string
  rule_hash: string
  account_template_id?: string | null
  account_snapshot: JsonRecord
  execution_snapshot: JsonRecord
  price_basis: "backward_adjusted"
  start_date: string
  end_date: string
  status: string
  dispatch_status: string
  nats_stream_sequence?: number | null
  summary: PortfolioSummary
  error_type?: string | null
  error_message?: string | null
  current_result_attempt_id?: string | null
}

export type PortfolioNavRecord = {
  portfolio_run_id: string
  trade_date: string
  cash_balance: number
  position_market_value: number
  total_equity: number
  nav: number
  daily_return?: number | null
  drawdown: number
  gross_exposure: number
  position_count: number
  turnover: number
  fee_amount: number
  warning_count: number
}

export type PortfolioTargetRecord = {
  portfolio_run_id: string
  signal_date: string
  execution_date: string
  security_code: string
  source_rank?: number | null
  source_score?: number | null
  target_weight: number
  target_amount: number
  target_quantity?: number | null
  target_reason: string
}

export type PortfolioOrderRecord = {
  portfolio_order_id: string
  portfolio_run_id: string
  order_seq: number
  signal_date?: string | null
  execution_date: string
  security_code: string
  side: string
  order_quantity: number
  order_amount: number
  reference_price?: number | null
  reason: string
  status: string
  event_ref?: string | null
}

export type PortfolioTradeRecord = {
  portfolio_trade_id: string
  portfolio_run_id: string
  trade_seq: number
  portfolio_order_id?: string | null
  trade_date: string
  signal_date?: string | null
  security_code: string
  side: string
  quantity: number
  reference_price: number
  execution_price: number
  gross_amount: number
  commission: number
  stamp_duty: number
  transfer_fee: number
  total_fee: number
  slippage_cost: number
  reason: string
}

export type PortfolioPositionRecord = {
  portfolio_run_id: string
  trade_date: string
  security_code: string
  quantity: number
  cost_basis: number
  average_entry_price: number
  close_price: number
  market_value: number
  unrealized_pnl: number
  unrealized_return: number
  holding_days: number
  is_stale_price: boolean
}

export type PortfolioEventRecord = {
  portfolio_event_id: string
  portfolio_run_id: string
  event_seq: number
  trade_date?: string | null
  security_code?: string | null
  event_type: string
  severity: string
  message: string
  payload: JsonRecord
}

export type RunChunkRecord = {
  run_id: string
  chunk_no: number
  start_date: string
  end_date: string
  status: string
  clickhouse_query_id?: string | null
  elapsed_ms?: number | null
  error_type?: string | null
  error_message?: string | null
}

export type RunDayRecord = {
  run_id: string
  trade_date: string
  status: string
  universe_count?: number | null
  pool_count?: number | null
  signal_count?: number | null
  error_type?: string | null
  error_message?: string | null
}

export type JsonRecord = Record<string, JsonValue>

export type PoolMemberRecord = {
  run_id: string
  trade_date: string
  security_code: string
  score?: number | null
  signal_rank?: number | null
  selected_metrics: JsonRecord
  filter_snapshot: JsonValue
}

export type BuySignalRecord = {
  run_id: string
  trade_date: string
  security_code: string
  rank: number
  score: number
  score_breakdown: JsonRecord
  selected_metrics: JsonRecord
}

export type MetricDefinition = {
  logical_metric: string
  mart_database: string
  mart_table: string
  column_name: string
  value_kind: "numeric" | "integer" | "boolean" | "string" | "date"
  allow_filter: boolean
  allow_scoring: boolean
  allowed_ops: Operator[]
  null_policy: "no_match" | "match" | "error"
  default_output: boolean
  description?: string | null
}

export type Operator =
  | "eq"
  | "ne"
  | "lt"
  | "lte"
  | "gt"
  | "gte"
  | "between"
  | "is_null"

export type RuleVersionSpec = {
  universe: UniverseSpec
  pool_filters: FilterExpr
  scoring: ScoringSpec
  top_n_default: number
  output_metrics: string[]
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

export type ListResult<T> = {
  items: T[]
  limit: number
  offset: number
  has_more: boolean
  total?: number
}

export type RunsQuery = {
  status?: string
  rule_set_id?: string
  start_date?: string
  end_date?: string
  keyword?: string
  limit?: number
  offset?: number
}

export type PortfolioRunsQuery = {
  source_run_id?: string
  status?: string
  dispatch_status?: string
  limit?: number
  offset?: number
}

export type PortfolioTargetQuery = {
  signal_date?: string
  limit?: number
  offset?: number
}

export type PortfolioOrderQuery = {
  execution_date?: string
  security_code?: string
  limit?: number
  offset?: number
}

export type PortfolioTradeQuery = {
  trade_date?: string
  security_code?: string
  limit?: number
  offset?: number
}

export type PortfolioPositionQuery = {
  trade_date?: string
  security_code?: string
  limit?: number
  offset?: number
}

export type PortfolioEventQuery = {
  trade_date?: string
  event_type?: string
  limit?: number
  offset?: number
}

export type RuleSetsQuery = {
  status?: string
  keyword?: string
  limit?: number
  offset?: number
}

export type RuleVersionsQuery = {
  status?: string
  limit?: number
  offset?: number
}

export type MetricsQuery = {
  mart_table?: string
  value_kind?: string
  allow_filter?: boolean
  allow_scoring?: boolean
  keyword?: string
}

export type ResultRowsQuery = {
  trade_date: string
  limit?: number
  offset?: number
  security_code?: string
  sort?: string
}

export type AnalysisSource = "signals" | "pool"

export type PriceAdjustment =
  | "forward_adjusted"
  | "backward_adjusted"
  | "unadjusted"

export type SecurityAnalysisQuery = {
  trade_date: string
  source: AnalysisSource
  adjustment?: PriceAdjustment
  quote_end_date?: string
  lookback_trading_days?: number
  quote_start_date?: string
  ma_windows?: string
}

export type ResultSnapshot = {
  rank?: number | null
  signal_rank?: number | null
  score?: number | null
  score_breakdown?: JsonValue
  selected_metrics: JsonRecord
  filter_snapshot?: JsonValue
}

export type SourceMetadata = {
  database: string
  table: string
  value_semantics: "current_mart_query" | string
  adjustment?: PriceAdjustment | null
}

export type AdjustedQuoteSourceMetadata = {
  database: string
  table: string
  value_semantics: "current_mart_query" | string
  adjustment_fields: PriceAdjustment[]
}

export type AnalysisSources = {
  quote: SourceMetadata
  adjusted_quote: AdjustedQuoteSourceMetadata
  trend: SourceMetadata
  momentum: SourceMetadata
}

export type ChartWindow = {
  start_date: string
  end_date: string
  lookback_trading_days: number
}

export type ChartOhlc = {
  open: number
  high: number
  low: number
  close: number
}

export type KdjSeries = {
  k?: number | null
  d?: number | null
  j?: number | null
  rsv?: number | null
}

export type RsiSeries = {
  "6"?: number | null
  "12"?: number | null
  "24"?: number | null
}

export type MacdSeries = {
  dif?: number | null
  dea?: number | null
  histogram?: number | null
}

export type BollSeries = {
  mid_20_2?: number | null
  up_20_2?: number | null
  dn_20_2?: number | null
}

export type ChartSeriesRow = {
  trade_date: string
  ohlc?: ChartOhlc | null
  volume?: number | null
  ma: Record<string, number | null | undefined>
  price_overlays?: Record<string, number | null | undefined>
  kdj: KdjSeries
  rsi: RsiSeries
  macd: MacdSeries
  boll: BollSeries
}

export type ChartMaMetadata = {
  requested_windows: number[]
  default_visible_windows: number[]
  available_windows: number[]
  adjustment: PriceAdjustment
  status: "available" | "forward_adjusted_only" | string
}

export type ChartPriceOverlayMetadata = {
  default_visible_keys: string[]
  available_keys: string[]
  adjustment: PriceAdjustment
  status: "available" | "forward_adjusted_only" | string
}

export type ChartPayload = {
  ma: ChartMaMetadata
  price_overlays?: ChartPriceOverlayMetadata
  indicator_panels: string[]
  series: ChartSeriesRow[]
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
  open_price_forward_adj?: number | null
  high_price_forward_adj?: number | null
  low_price_forward_adj?: number | null
  close_price_forward_adj?: number | null
  prev_close_price_forward_adj?: number | null
  open_price_backward_adj?: number | null
  high_price_backward_adj?: number | null
  low_price_backward_adj?: number | null
  close_price_backward_adj?: number | null
  prev_close_price_backward_adj?: number | null
  forward_adjustment_factor?: number | null
  forward_adjustment_ratio?: number | null
  backward_adjustment_factor?: number | null
  backward_adjustment_ratio?: number | null
  prev_volume?: number | null
  volume?: number | null
  amount?: number | null
  turnover_rate?: number | null
  turnover_rate_actual?: number | null
  pct_amplitude?: number | null
  pct_change?: number | null
  limit_up_price?: number | null
  limit_down_price?: number | null
  a_market_cap?: number | null
  a_float_market_cap?: number | null
  a_free_float_market_cap?: number | null
  a_shares?: number | null
  a_float_shares?: number | null
  a_free_float_shares?: number | null
  pe_static?: number | null
  pe_ttm?: number | null
  pe_forecast?: number | null
  pb_mrq?: number | null
  book_value_per_share?: number | null
  roe?: number | null
  roa?: number | null
  roaa?: number | null
  roae?: number | null
  dy_static?: number | null
  dy_ttm?: number | null
  is_suspend?: boolean | null
  is_st?: boolean | null
  kdj_rsv?: number | null
  kdj_k_value?: number | null
  kdj_d_value?: number | null
  kdj_j_value?: number | null
}

export type SecurityAnalysisResponse = {
  run_id: string
  trade_date: string
  security_code: string
  source: AnalysisSource
  adjustment: PriceAdjustment
  result_snapshot: ResultSnapshot
  sources: AnalysisSources
  chart_window: ChartWindow
  chart: ChartPayload
  quote_rows: QuoteMartRow[]
  selected_quote?: QuoteMartRow | null
}

export type CreateRuleSetRequest = {
  name: string
  description?: string
  owner?: string
  tags?: string[]
}

export type CreateRuleVersionRequest = {
  rule: RuleVersionSpec
  activate?: boolean
  created_by?: string
}

export type CreateRunRequest = {
  rule_set_id?: string
  rule_version_id?: string
  start_date: string
  end_date: string
  top_n?: number
  universe_snapshot?: JsonValue
}

export type CreatePortfolioRunRequest = {
  source_run_id: string
  account_template_id?: string
}

export type CreateAccountTemplateRequest = {
  market?: string
  name?: string
  initial_cash?: number
  currency?: string
  fee_profile?: FeeProfile
  slippage_profile?: SlippageProfile
  rebalance_policy?: JsonRecord
  risk_exit_policy?: JsonRecord
  is_default?: boolean
}

export type PatchAccountTemplateRequest = {
  name?: string
  initial_cash?: number
  currency?: string
  fee_profile?: FeeProfile
  slippage_profile?: SlippageProfile
  rebalance_policy?: JsonRecord
  risk_exit_policy?: JsonRecord
  is_default?: boolean
  status?: string
}
