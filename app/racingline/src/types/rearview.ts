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

export type StrategyPreviewOpenRequest = {
  rule: RuleVersionSpec
  start_date: string
  end_date: string
  preview_row_limit: number
  top_n?: number
}

export type StrategyPreviewOpenResponse = {
  preview_id: string
  sql_hash: string
  required_metrics: string[]
  required_marts: string[]
  required_columns: Record<string, string[]>
  timeline: {
    start_date: string
    end_date: string
    trade_dates: StrategyPreviewTimelineTradeDate[]
  }
  latest?: StrategyPreviewTradeDate | null
  preview_row_limit: number
  top_n: number
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

export type BacktestDateRange = {
  start_date: string
  end_date: string
}

export type BacktestExecutionConfig = {
  market: "CN_A_SHARE"
  account: BacktestAccountConfig
  signal_policy: BacktestSignalPolicy
  rebalance_policy: BacktestRebalancePolicy
  fee_profile: FeeProfile
  slippage_profile: BacktestSlippageProfile
  risk_exit_policy: BacktestRiskExitPolicy
  price_basis: "backward_adjusted"
}

export type BacktestAccountConfig = {
  initial_cash: number
  currency: "CNY"
}

export type BacktestSignalPolicy = {
  buy_signal_top_n: number
  signal_timing: "close_confirm_next_open"
}

export type BacktestRebalancePolicy = {
  target_weighting: "equal_weight_capped"
  max_positions: number
  single_position_limit_pct: number
  cash_reserve_pct: number
  lot_size: 100
  min_trade_lots: 1
  empty_signal_action: "hold"
}

export type BacktestSlippageProfile = {
  mode: "bps"
  buy_bps: number
  sell_bps: number
}

export type BacktestRiskExitPolicy = {
  trigger_timing: "close_confirm_next_open"
  exit_rules: ExitRuleConfig[]
}

export type ExitRuleConfig =
  | {
      type: "fixed_stop_loss"
      loss_pct: number
    }
  | {
      type: "take_profit"
      profit_pct: number
    }
  | {
      type: "time_stop_loss"
      holding_days: number
      max_return_pct: number
    }
  | {
      type: "indicator_stop_loss"
      source: "trend"
      metric: string
      operator: "close_below_metric"
    }

export type StrategyBacktestValidateRequest = {
  rule: RuleVersionSpec
  preview_id?: string
  preview_range?: BacktestDateRange
  execution_config: BacktestExecutionConfig
  range?: BacktestDateRange
  benchmark?: string
}

export type BacktestExecutionSummary = {
  buy_signal_top_n: number
  max_positions: number
  target_weight_per_position_pct: number
  implicit_cash_reserve_pct: number
  enabled_exit_rule_count: number
}

export type StrategyBacktestConfigSummary = BacktestExecutionSummary
export type StrategyBacktestProgress = JsonRecord

export type StrategyBacktestDraftResponse = {
  preview_id?: string
  preview_range?: BacktestDateRange
  range?: BacktestDateRange
  benchmark?: string
  execution_config: BacktestExecutionConfig
  rule_hash: string
  execution_config_hash: string
  summary: BacktestExecutionSummary
  warnings: string[]
}

export type StrategyBacktestPeriodKey = "1y" | "2y" | "3y"

export type StrategyBacktestBenchmarkOption = {
  security_code: string
  label: string
  is_default: boolean
  availability_status: string
}

export type StrategyBacktestPeriodOption = {
  period_key: StrategyBacktestPeriodKey
  label: string
  resolved_start_date: string
  resolved_end_date: string
  latest_available_trade_date: string
  benchmark_security_code: string
  range_resolution_snapshot: JsonValue
}

export type StrategyBacktestOptionsResponse = {
  default_period_key: StrategyBacktestPeriodKey
  default_benchmark_security_code: string
  selected_benchmark_security_code: string
  as_of_date: string
  latest_available_trade_date: string
  period_options: StrategyBacktestPeriodOption[]
  benchmark_options: StrategyBacktestBenchmarkOption[]
  range_resolution_snapshot: JsonValue
}

export type StrategyBacktestCreateRequest = {
  rule: RuleVersionSpec
  period_key: StrategyBacktestPeriodKey
  benchmark_security_code: string
  execution_config: BacktestExecutionConfig
  preview_id?: string
  preview_range?: BacktestDateRange
  top_n?: number
  rule_hash?: string
  execution_config_hash?: string
  client_request_id?: string
  ui_display_snapshot?: JsonRecord
  range_hint?: BacktestDateRange
}

export type StrategyBacktestRunStatus =
  | "created"
  | "queued"
  | "compiling_signals"
  | "running_clickhouse"
  | "loading_market_data"
  | "calculating_nav"
  | "computing_performance"
  | "writing_results"
  | "succeeded"
  | "failed_validation"
  | "failed_compile"
  | "failed_market_data"
  | "failed_simulation"
  | "failed_write"
  | "cancelled"

export type StrategyBacktestRunRecord = {
  strategy_backtest_run_id: string
  rule_snapshot: JsonValue
  rule_hash: string
  execution_config: BacktestExecutionConfig
  execution_config_hash: string
  catalog_hash?: string | null
  compiled_sql_hash?: string | null
  required_metrics: JsonValue
  required_marts: JsonValue
  data_preflight_snapshot: JsonValue
  preview_id?: string | null
  preview_range?: JsonValue | null
  period_key: StrategyBacktestPeriodKey
  range_as_of_date?: string | null
  range_resolved_at?: string | null
  range_resolution_snapshot: JsonValue
  start_date: string
  end_date: string
  benchmark_security_code: string
  price_basis: "backward_adjusted"
  ui_display_snapshot: JsonValue
  client_request_id?: string | null
  request_hash: string
  status: StrategyBacktestRunStatus
  dispatch_status: "pending" | "published" | "publish_failed"
  nats_stream_sequence?: number | null
  worker_attempt_no: number
  claimed_at?: string | null
  heartbeat_at?: string | null
  claim_expires_at?: string | null
  progress: StrategyBacktestProgress
  summary: JsonValue
  signal_summary: JsonValue
  data_coverage_summary: JsonValue
  error_type?: string | null
  error_message?: string | null
  current_result_attempt_id?: string | null
  config_summary: StrategyBacktestConfigSummary
}

export type StrategyBacktestRunStatusView = {
  strategy_backtest_run_id: string
  status: StrategyBacktestRunStatus
  dispatch_status: "pending" | "published" | "publish_failed"
  progress: StrategyBacktestProgress
  error_type?: string | null
  error_message?: string | null
  period_key: StrategyBacktestPeriodKey
  benchmark_security_code: string
  start_date: string
  end_date: string
  rule_hash: string
  execution_config_hash: string
  current_result_attempt_id?: string | null
}

export type StrategyBacktestNavPoint = {
  trade_date: string
  strategy_nav: number
  benchmark_nav?: number | null
  excess_return?: number | null
}

export type StrategyBacktestOverviewUiResponse = {
  status: StrategyBacktestRunStatusView
  latest_nav?: StrategyBacktestNavPoint | null
  nav_points: StrategyBacktestNavPoint[]
  performance: StrategyBacktestPerformanceUiView
  rebalance: StrategyBacktestRebalanceRecordsUiResponse
}

export type StrategyPortfolioLiveStatus =
  | "pending_first_run"
  | "queued"
  | "running"
  | "succeeded"
  | "failed"

export type StrategyPortfolioCurveSource =
  | "none"
  | "publish_preview"
  | "source_backtest"
  | "live_daily_run"

export type StrategyPortfolioRecord = {
  strategy_portfolio_id: string
  portfolio_code: string
  name: string
  status: "active" | "archived"
  rule_snapshot: JsonValue
  rule_hash: string
  execution_config: BacktestExecutionConfig
  execution_config_hash: string
  benchmark_security_code: string
  price_basis: "backward_adjusted"
  catalog_hash?: string | null
  required_metrics: JsonValue
  required_marts: JsonValue
  source_strategy_backtest_run_id: string
  source_result_attempt_id: string
  source_period_key: StrategyBacktestPeriodKey
  source_start_date: string
  source_end_date: string
  initial_signal_date: string
  live_start_date: string
  pending_buy_signal_snapshot: StrategyPortfolioPendingBuySignal[]
  latest_daily_run_id?: string | null
  current_result_attempt_id?: string | null
  current_live_result_attempt_id?: string | null
  ui_display_snapshot: JsonValue
  client_request_id?: string | null
  request_hash: string
  created_at: string
  updated_at: string
  archived_at?: string | null
  live_status: StrategyPortfolioLiveStatus
  backtest_segment: StrategyPortfolioBacktestSegment
  live_segment: StrategyPortfolioLiveSegment
}

export type StrategyPortfolioCreateRequest = {
  source_strategy_backtest_run_id: string
  source_result_attempt_id: string
  name: string
  expected_required_source_signal_date: string
  expected_source_signal_date: string
  expected_live_start_date: string
  client_request_id?: string
}

export type StrategyPortfolioPendingBuySignal = {
  security_code: string
  security_name?: string | null
  source_rank: number
  source_score: number
  signal_date: string
  execution_date: string
}

export type StrategyPortfolioPublishPreviewResponse = {
  can_publish: boolean
  blockers: string[]
  source_strategy_backtest_run_id: string
  source_result_attempt_id: string
  source_signal_date: string
  server_current_date: string
  server_current_time: string
  market_phase: "before_close" | "after_close" | "non_trading_day"
  publish_cutoff_time: string
  required_source_signal_date: string
  planned_live_start_date?: string | null
  source_period_key: StrategyBacktestPeriodKey
  source_start_date: string
  source_end_date: string
  benchmark_security_code: string
  pending_buy_signals: StrategyPortfolioPendingBuySignal[]
}

export type StrategyPortfolioBacktestSegment = {
  source_strategy_backtest_run_id: string
  source_result_attempt_id: string
  period_key: StrategyBacktestPeriodKey
  start_date: string
  end_date: string
  benchmark_security_code: string
}

export type StrategyPortfolioLiveSegment = {
  live_status: StrategyPortfolioLiveStatus
  live_start_date: string
  initial_signal_date: string
  latest_daily_run_id?: string | null
  current_live_result_attempt_id?: string | null
  performance_source: "none" | "live_daily_run"
  signal_source: "publish_preview" | "live_daily_run"
}

export type StrategyPortfolioDashboardCard = {
  strategy_portfolio_id: string
  portfolio_code: string
  name: string
  status: "active" | "archived"
  live_status: StrategyPortfolioLiveStatus
  curve_source: StrategyPortfolioCurveSource
  latest_daily_run_id?: string | null
  current_result_attempt_id?: string | null
  source_strategy_backtest_run_id: string
  source_result_attempt_id: string
  source_period_key: StrategyBacktestPeriodKey
  source_start_date: string
  source_end_date: string
  initial_signal_date: string
  live_start_date: string
  backtest_segment: StrategyPortfolioBacktestSegment
  live_segment: StrategyPortfolioLiveSegment
  source_backtest_summary: JsonValue
  live_summary?: JsonValue | null
  ui_display_snapshot: JsonValue
  latest_nav?: number | null
  recent_change?: number | null
  returns: {
    label: string
    value?: number | null
    kind: "percent" | "ratio"
    tone?: "up" | "down" | "neutral"
  }[]
  risk: {
    label: string
    value?: number | null
    kind: "percent" | "ratio"
    tone?: "up" | "down" | "neutral"
  }[]
  efficiency: {
    label: string
    value?: number | null
    kind: "percent" | "ratio"
    tone?: "up" | "down" | "neutral"
  }[]
  relative: {
    label: string
    value?: number | null
    kind: "percent" | "ratio"
    tone?: "up" | "down" | "neutral"
  }[]
  today_signals: {
    code: string
    name: string
    score: number
    rank: number
    signal_date: string
    execution_date: string
  }[]
  pending_buy_signals: {
    code: string
    name: string
    score: number
    rank: number
    signal_date: string
    execution_date: string
  }[]
  curve: {
    time: string
    nav: number
    benchmark: number
  }[]
  created_at: string
  updated_at: string
}

export type StrategyPortfolioDashboardResponse = {
  portfolios: StrategyPortfolioDashboardCard[]
}

export type StrategyPortfolioNavResponse = {
  source: StrategyPortfolioCurveSource
  points: StrategyBacktestNavPoint[]
}

export type StrategyPortfolioPerformanceView =
  StrategyBacktestPerformanceView & {
    source: StrategyPortfolioCurveSource
  }

export type StrategyPortfolioVirtualAccount = {
  source: "live_daily_run"
  strategy_portfolio_id: string
  strategy_portfolio_daily_run_id: string
  result_attempt_id: string
  account_date: string
  currency: "CNY"
  total_equity: number
  position_market_value: number
  cash_balance: number
  holding_unrealized_pnl: number
  daily_pnl?: number | null
  daily_return?: number | null
  position_count: number
}

export type StrategyPortfolioStatementPeriodKey =
  | "month"
  | "three_months"
  | "six_months"
  | "ytd"
  | "all"

export type StrategyPortfolioStatementQuery = {
  period: StrategyPortfolioStatementPeriodKey
  limit?: number
  offset?: number
}

export type StrategyPortfolioStatementResponse = {
  source: "live_daily_run"
  strategy_portfolio_id: string
  strategy_portfolio_daily_run_id: string
  result_attempt_id: string
  period: {
    key: StrategyPortfolioStatementPeriodKey
    label: string
    start_date: string
    end_date: string
    latest_live_trade_date: string
  }
  summary: {
    average_position_pct?: number | null
    traded_security_count: number
    trade_count: number
    trade_win_rate?: number | null
    winning_security_count: number
    losing_security_count: number
    holding_days: number
  }
  operations: {
    items: StrategyPortfolioStatementOperation[]
    limit: number
    offset: number
    has_more: boolean
  }
}

export type StrategyPortfolioStatementOperation = {
  portfolio_trade_id: string
  trade_seq: number
  trade_date: string
  security_code: string
  security_name?: string | null
  side: "buy" | "sell"
  execution_price: number
  quantity: number
  lot_size: number
  lot_count: number
  gross_amount: number
  commission: number
  stamp_duty: number
  transfer_fee: number
  total_fee: number
  position_balance_quantity: number
  realized_pnl?: number | null
  reason: string
}

export type StrategyPortfolioListResult<T> = ListResult<T> & {
  source: StrategyPortfolioCurveSource
}

export type StrategyPortfolioSignalsResponse =
  StrategyPortfolioListResult<StrategyBacktestTargetRecord> & {
    signal_source: "publish_preview" | "live_daily_run"
    pending_buy_signals: StrategyPortfolioDashboardCard["pending_buy_signals"]
  }

export type StrategyPortfolioSignalTimelinePoint = {
  trade_date: string
  target_count: number
  signal_count?: number | null
}

export type StrategyPortfolioSignalTimelineResponse = {
  source: StrategyPortfolioCurveSource
  signal_source: "publish_preview" | "live_daily_run"
  trade_dates: StrategyPortfolioSignalTimelinePoint[]
}

export type StrategyPortfolioRebalanceRecordsResponse =
  StrategyBacktestRebalanceRecordsResponse & {
    source: StrategyPortfolioCurveSource
  }

export type StrategyBacktestRebalanceRow = {
  direction: "buy" | "hold" | "sell"
  security_code: string
  security_name?: string | null
  quantity: number
  holding_days?: number | null
  change_pct?: number | null
  cost_price?: number | null
  current_price?: number | null
  contribution_pct?: number | null
  reason?: string | null
}

export type StrategyBacktestRebalanceUiRow = Omit<
  StrategyBacktestRebalanceRow,
  "quantity" | "reason"
>

export type StrategyBacktestRebalanceRecord = {
  trade_date: string
  position_count: number
  buy_count: number
  hold_count: number
  sell_count: number
  rows: StrategyBacktestRebalanceRow[]
}

export type StrategyBacktestRebalanceRecordSummary = Omit<
  StrategyBacktestRebalanceRecord,
  "rows"
>

export type StrategyBacktestRebalanceRecordsResponse = {
  selected_trade_date: string
  records: StrategyBacktestRebalanceRecord[]
}

export type StrategyBacktestRebalanceRecordsUiResponse = {
  selected_trade_date: string
  records: StrategyBacktestRebalanceRecordSummary[]
  selected_rows: StrategyBacktestRebalanceUiRow[]
}

export type StrategyBacktestPerformanceUiView = {
  metric: JsonRecord
  daily_win_rate: {
    value?: number | null
    observation_count: number
    winning_day_count: number
  }
}

export type StrategyBacktestPerformanceView =
  StrategyBacktestPerformanceUiView & {
    statuses: JsonRecord[]
  }

export type StrategyBacktestTargetRecord = {
  portfolio_run_id: string
  signal_date: string
  execution_date: string
  security_code: string
  security_name?: string | null
  source_rank?: number | null
  source_score?: number | null
  target_weight: number
  target_amount: number
  target_quantity?: number | null
  target_reason: string
}

export type StrategyBacktestOrderRecord = {
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

export type StrategyBacktestTradeRecord = {
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

export type StrategyBacktestPositionRecord = {
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

export type StrategyBacktestEventRecord = {
  portfolio_event_id: string
  portfolio_run_id: string
  event_seq: number
  trade_date?: string | null
  security_code?: string | null
  event_type: string
  severity: string
  message: string
  payload: JsonValue
}

export type StrategyBacktestClosedTradeRecord = {
  portfolio_run_id: string
  result_attempt_id: string
  closed_trade_id: string
  closed_trade_seq: number
  position_lot_id: string
  entry_trade_seq: number
  exit_trade_seq: number
  security_code: string
  entry_date: string
  exit_date: string
  quantity: number
  entry_gross_amount: number
  exit_gross_amount: number
  entry_fee: number
  exit_fee: number
  total_fee: number
  realized_pnl: number
  realized_return?: number | null
  holding_days: number
  exit_reason: string
}

export type StrategyBacktestTradeMetricRecord = {
  portfolio_run_id: string
  result_attempt_id: string
  window_key: string
  window_start?: string | null
  window_end?: string | null
  closed_trade_count: number
  winning_trade_count: number
  losing_trade_count: number
  breakeven_trade_count: number
  win_rate_closed_trades?: number | null
  average_win_return?: number | null
  average_loss_return?: number | null
  profit_loss_ratio?: number | null
  average_holding_days?: number | null
  largest_win_return?: number | null
  largest_loss_return?: number | null
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

export type PreviewChartContextSeriesRow = {
  trade_date: string
  ohlc?: ChartOhlc | null
  volume?: number | null
  ma: Record<string, number | null>
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

export type PreviewChartContextQuote = {
  trade_date: string
  open_price?: number | null
  high_price?: number | null
  low_price?: number | null
  close_price?: number | null
  prev_close_price?: number | null
  pct_change?: number | null
  pct_amplitude?: number | null
  volume?: number | null
  amount?: number | null
  limit_up_price?: number | null
  limit_down_price?: number | null
  a_market_cap?: number | null
  pe_ttm?: number | null
  roe?: number | null
}

export type SecurityAnalysisRequest = {
  trade_date: string
  security_code: string
  adjustment?: Adjustment
  quote_end_date?: string
  quote_start_date?: string
  lookback_trading_days?: number
  ma_windows?: string
  include_quote_rows?: boolean
}

export type PreviewChartContextRequest = {
  trade_date: string
  security_code: string
  adjustment?: Adjustment
  lookback_trading_days?: number
  ma_windows?: string
}

export type PreviewChartContextResponse = {
  security_code: string
  security_name?: string | null
  security_board?: string | null
  chart: {
    ma: {
      available_windows: number[]
    }
    series: PreviewChartContextSeriesRow[]
  }
  selected_quote?: PreviewChartContextQuote | null
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
  result_snapshot?: {
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
