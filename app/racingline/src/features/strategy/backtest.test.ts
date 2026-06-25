import { describe, expect, it } from "vitest"

import {
  acceptStrategyBacktestRunForStep5,
  hasStrategyBacktestConfigChanged,
  isStrategyBacktestResultReady,
  mergeStrategyBacktestStatus,
} from "@/features/strategy/backtest"
import type { BacktestExecutionDraft } from "@/features/strategy/execution"
import type {
  BacktestExecutionConfig,
  BacktestExecutionSummary,
  RuleVersionSpec,
  StrategyBacktestRunRecord,
  StrategyBacktestRunStatusView,
} from "@/types/rearview"

const ruleSpec: RuleVersionSpec = {
  universe: {
    base: "cn_a_share",
    exclude_st: true,
    exclude_suspend: true,
    include_security_codes: [],
    exclude_security_codes: [],
  },
  pool_filters: { type: "all", conditions: [] },
  scoring: {
    rules: [],
    clamp: { min: 0, max: 100 },
  },
  top_n_default: 5,
  output_metrics: [],
}

const executionConfig: BacktestExecutionConfig = {
  market: "CN_A_SHARE",
  account: {
    currency: "CNY",
    initial_cash: 1_000_000,
  },
  signal_policy: {
    buy_signal_top_n: 5,
    signal_timing: "close_confirm_next_open",
  },
  rebalance_policy: {
    cash_reserve_pct: 0,
    empty_signal_action: "hold",
    lot_size: 100,
    max_positions: 5,
    min_trade_lots: 1,
    single_position_limit_pct: 10,
    target_weighting: "equal_weight_capped",
  },
  fee_profile: {
    commission_rate: 0.0001,
    commission_rate_max: 0.0001,
    min_commission: 5,
    stamp_duty_rate_sell: 0.0005,
    transfer_fee_rate: 0.00001,
  },
  slippage_profile: {
    buy_bps: 10,
    mode: "bps",
    sell_bps: 10,
  },
  risk_exit_policy: {
    trigger_timing: "close_confirm_next_open",
    exit_rules: [],
  },
  price_basis: "backward_adjusted",
}

const configSummary: BacktestExecutionSummary = {
  buy_signal_top_n: 5,
  enabled_exit_rule_count: 0,
  implicit_cash_reserve_pct: 0,
  max_positions: 5,
  target_weight_per_position_pct: 20,
}

function buildDraft(
  overrides: Partial<BacktestExecutionDraft> = {}
): BacktestExecutionDraft {
  return {
    appliedRuleSpec: ruleSpec,
    createdAt: "2026-06-25T00:00:00Z",
    execution_config: executionConfig,
    execution_config_hash: "config-hash",
    rule_hash: "rule-hash",
    stale: false,
    summary: configSummary,
    warnings: [],
    ...overrides,
  }
}

function buildRun(
  overrides: Partial<StrategyBacktestRunRecord> = {}
): StrategyBacktestRunRecord {
  return {
    benchmark_security_code: "000300.SH",
    catalog_hash: "catalog-hash",
    claim_expires_at: null,
    claimed_at: null,
    client_request_id: "client-request-1",
    compiled_sql_hash: null,
    config_summary: configSummary,
    current_result_attempt_id: null,
    data_coverage_summary: { price_bar_count: 0 },
    data_preflight_snapshot: { risk_free_return_count: 0 },
    dispatch_status: "pending",
    end_date: "2025-12-31",
    error_message: null,
    error_type: null,
    execution_config: executionConfig,
    execution_config_hash: "config-hash",
    heartbeat_at: null,
    nats_stream_sequence: null,
    period_key: "1y",
    preview_id: "preview-1",
    preview_range: { start_date: "2025-01-02", end_date: "2025-12-31" },
    price_basis: "backward_adjusted",
    progress: { stage: "queued" },
    range_as_of_date: "2025-12-31",
    range_resolution_snapshot: { method: "test" },
    range_resolved_at: null,
    request_hash: "request-hash",
    required_marts: [],
    required_metrics: [],
    rule_hash: "rule-hash",
    rule_snapshot: { rules: ["full-snapshot"] },
    signal_summary: { signal_count: 0 },
    start_date: "2025-01-02",
    status: "queued",
    strategy_backtest_run_id: "run-1",
    summary: { worker_timing: { total_ms: 0 } },
    ui_display_snapshot: { source: "full-run" },
    worker_attempt_no: 0,
    ...overrides,
  }
}

function buildStatus(
  overrides: Partial<StrategyBacktestRunStatusView> = {}
): StrategyBacktestRunStatusView {
  return {
    benchmark_security_code: "000300.SH",
    current_result_attempt_id: null,
    dispatch_status: "published",
    end_date: "2025-12-31",
    error_message: null,
    error_type: null,
    execution_config_hash: "config-hash",
    period_key: "1y",
    progress: { stage: "running_clickhouse" },
    rule_hash: "rule-hash",
    start_date: "2025-01-02",
    status: "running_clickhouse",
    strategy_backtest_run_id: "run-1",
    ...overrides,
  }
}

describe("strategy backtest handoff", () => {
  it("enters Step 5 for an accepted queued run without waiting for terminal status", () => {
    const run = buildRun({ status: "queued" })

    expect(acceptStrategyBacktestRunForStep5(run)).toEqual({
      activeRun: run,
      activeStep: "backtest",
    })
  })
})

describe("strategy backtest result gate", () => {
  it("does not load result wrappers while the run is still active", () => {
    expect(
      isStrategyBacktestResultReady(
        buildStatus({ status: "running_clickhouse" }),
        buildDraft(),
        "1y",
        "000300.SH"
      )
    ).toBe(false)
  })

  it("loads result wrappers only for the current succeeded matching run", () => {
    expect(
      isStrategyBacktestResultReady(
        buildStatus({
          current_result_attempt_id: "attempt-1",
          status: "succeeded",
        }),
        buildDraft(),
        "1y",
        "000300.SH"
      )
    ).toBe(true)
  })

  it("does not load stale results after config changes", () => {
    expect(
      hasStrategyBacktestConfigChanged(
        buildStatus({
          current_result_attempt_id: "attempt-1",
          status: "succeeded",
        }),
        buildDraft({ execution_config_hash: "new-config-hash" }),
        "1y",
        "000300.SH"
      )
    ).toBe(true)

    expect(
      isStrategyBacktestResultReady(
        buildStatus({
          current_result_attempt_id: "attempt-1",
          status: "succeeded",
        }),
        buildDraft({ execution_config_hash: "new-config-hash" }),
        "1y",
        "000300.SH"
      )
    ).toBe(false)
  })
})

describe("strategy backtest status merge", () => {
  it("keeps the same object when the lightweight status is unchanged", () => {
    const run = buildRun()

    expect(
      mergeStrategyBacktestStatus(
        run,
        buildStatus({
          dispatch_status: run.dispatch_status,
          progress: run.progress,
          status: run.status,
        })
      )
    ).toBe(run)
  })

  it("updates lightweight status fields without replacing full run snapshots", () => {
    const run = buildRun()
    const merged = mergeStrategyBacktestStatus(
      run,
      buildStatus({
        current_result_attempt_id: "attempt-1",
        progress: { stage: "succeeded" },
        status: "succeeded",
      })
    )

    expect(merged.status).toBe("succeeded")
    expect(merged.current_result_attempt_id).toBe("attempt-1")
    expect(merged.rule_snapshot).toEqual({ rules: ["full-snapshot"] })
    expect(merged.execution_config).toBe(executionConfig)
    expect(merged.summary).toEqual({ worker_timing: { total_ms: 0 } })
  })

  it("ignores a status update for a different run id", () => {
    const run = buildRun()

    expect(
      mergeStrategyBacktestStatus(
        run,
        buildStatus({ strategy_backtest_run_id: "run-2" })
      )
    ).toBe(run)
  })
})
