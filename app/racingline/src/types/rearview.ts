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
