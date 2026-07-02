import { describe, expect, it } from "vitest"

import {
  buildStrategyConfigDisplayFromCanonical,
  buildStrategyConfigDisplayFromDraft,
  buildStrategyConfigSourceContext,
  readRuleVersionSpec,
} from "@/features/strategy/config-display"
import type {
  IndicatorCatalog,
  SimulationSettings,
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import type {
  BacktestExecutionConfig,
  RuleVersionSpec,
} from "@/types/rearview"

const catalog: IndicatorCatalog[] = [
  {
    id: "quotes",
    label: "行情",
    source: "quotes",
    metrics: [
      {
        allowedOps: ["gt", "gte"],
        id: "close_price",
        label: "收盘价",
        valueType: "number",
      },
      {
        allowedOps: ["gt"],
        id: "volume_ratio",
        label: "量比",
        valueType: "number",
      },
      {
        allowedOps: ["between"],
        id: "turnover_rate",
        label: "换手率",
        valueType: "number",
      },
    ],
  },
  {
    id: "trend",
    label: "趋势",
    source: "trend",
    metrics: [
      {
        allowedOps: ["gt", "lt"],
        id: "price_ma_10",
        label: "价格 MA10",
        valueType: "number",
      },
      {
        allowedOps: ["gt", "lt"],
        id: "price_ma_20",
        label: "价格 MA20",
        valueType: "number",
      },
    ],
  },
  {
    id: "momentum",
    label: "动量",
    source: "momentum",
    metrics: [
      {
        allowedOps: ["lt"],
        id: "kdj_j_value",
        label: "KDJ J",
        valueType: "number",
      },
    ],
  },
]

const conditionGroups: StrategyConditionGroup[] = [
  {
    id: "group-1",
    name: "价格组",
    conditions: [
      {
        catalogId: "quotes",
        compareCatalogId: "quotes",
        compareMetric: "volume_ratio",
        id: "condition-1",
        logic: "and",
        metric: "close_price",
        operator: "gt",
        target: "value",
        value: "10",
        valueEnd: "",
      },
      {
        catalogId: "quotes",
        compareCatalogId: "quotes",
        compareMetric: "close_price",
        id: "condition-2",
        logic: "or",
        metric: "volume_ratio",
        operator: "gte",
        target: "value",
        value: "1.2",
        valueEnd: "",
      },
    ],
  },
]

const weightIndicators: WeightIndicator[] = [
  {
    catalogId: "quotes",
    compareCatalogId: "quotes",
    compareMetric: "volume_ratio",
    id: "weight-1",
    metric: "close_price",
    operator: "gt",
    score: 30,
    target: "value",
    value: "12",
    valueEnd: "",
  },
]

const settings: SimulationSettings = {
  buyTopN: 10,
  fixedStopLoss: {
    enabled: true,
    lossPercent: 8,
  },
  indicatorStopLoss: {
    catalogId: "trend",
    enabled: false,
    metric: "price_ma_10",
  },
  initialCapital: 1_000_000,
  maxPositions: 10,
  singlePositionLimitPercent: 10,
  takeProfit: {
    enabled: false,
    profitPercent: 20,
  },
  timeStopLoss: {
    enabled: true,
    holdingDays: 20,
    minimumReturnPercent: 0,
  },
  transactionFees: {
    commissionRatePercent: 0.01,
    slippageRatePercent: 0.1,
    stampDutyRatePercent: 0.05,
    transferFeeRatePercent: 0.001,
  },
}

const canonicalRule: RuleVersionSpec = {
  universe: {
    base: "all_a_shares",
    exclude_st: true,
    exclude_suspend: true,
    include_security_codes: [],
    exclude_security_codes: [],
  },
  pool_filters: {
    type: "all",
    conditions: [
      {
        type: "compare",
        left: { type: "metric", name: "close_price" },
        op: "gt",
        right: { type: "metric", name: "price_ma_20" },
      },
      {
        type: "compare",
        left: { type: "metric", name: "turnover_rate" },
        op: "between",
        right: {
          type: "range",
          min: { type: "number", value: 2 },
          max: { type: "number", value: 8 },
        },
      },
    ],
  },
  scoring: {
    rules: [
      {
        type: "conditional_points",
        name: "kdj-low",
        condition: {
          type: "compare",
          left: { type: "metric", name: "kdj_j_value" },
          op: "lt",
          right: { type: "number", value: -10 },
        },
        points: 25,
      },
    ],
    clamp: { min: 0, max: 100 },
  },
  top_n_default: 10,
  output_metrics: ["close_price", "turnover_rate", "kdj_j_value"],
}

const canonicalExecutionConfig: BacktestExecutionConfig = {
  market: "CN_A_SHARE",
  account: {
    initial_cash: 1_000_000,
    currency: "CNY",
  },
  signal_policy: {
    buy_signal_top_n: 5,
    signal_timing: "close_confirm_next_open",
  },
  rebalance_policy: {
    target_weighting: "equal_weight_capped",
    max_positions: 8,
    single_position_limit_pct: 0.1,
    cash_reserve_pct: 0,
    lot_size: 100,
    min_trade_lots: 1,
    empty_signal_action: "hold",
  },
  fee_profile: {
    commission_rate: 0.0001,
    commission_rate_max: 0.003,
    min_commission: 5,
    stamp_duty_rate_sell: 0.0005,
    transfer_fee_rate: 0.00001,
  },
  slippage_profile: {
    mode: "bps",
    buy_bps: 10,
    sell_bps: 12,
  },
  risk_exit_policy: {
    trigger_timing: "close_confirm_next_open",
    exit_rules: [
      { type: "fixed_stop_loss", loss_pct: 0.08 },
      { type: "take_profit", profit_pct: 0.2 },
      { type: "time_stop_loss", holding_days: 20, max_return_pct: 0 },
      {
        type: "indicator_stop_loss",
        source: "trend",
        metric: "price_ma_10",
        operator: "close_below_metric",
      },
    ],
  },
  price_basis: "backward_adjusted",
}

describe("buildStrategyConfigDisplayFromDraft", () => {
  it("builds Step 5 compatible condition, scoring and summary rows", () => {
    const display = buildStrategyConfigDisplayFromDraft({
      conditionCatalogOptions: catalog,
      conditionGroups,
      scoringCatalogOptions: catalog,
      settings,
      weightIndicators,
    })

    expect(display.conditionRows).toEqual([
      {
        expression: "收盘价 > 10",
        groupLabel: "价格组",
        id: "condition-1",
        logicLabel: "组内起始",
      },
      {
        expression: "量比 >= 1.2",
        groupLabel: "价格组",
        id: "condition-2",
        logicLabel: "OR",
      },
    ])
    expect(display.scoringRows).toEqual([
      {
        expression: "收盘价 > 12",
        id: "weight-1",
        index: 1,
        score: 30,
      },
    ])
    expect(display.buildSummaryRows).toEqual([
      { label: "初始资金", value: "¥1000000.00" },
      { label: "每日候选", value: "Top 10" },
      { label: "候选口径", value: "Top N 是每日候选信号，不是目标持仓集合" },
      { label: "调仓规则", value: "仅空位调入；旧持仓由风控退出" },
      { label: "最大持仓", value: "10 只" },
      { label: "单票上限", value: "10%" },
      { label: "交易成本", value: "佣金 0.01% / 滑点 0.1%" },
      { label: "风控", value: "固定止损 8%，时间止损 20 天" },
    ])
  })
})

describe("buildStrategyConfigDisplayFromCanonical", () => {
  it("derives display rows from RuleVersionSpec and BacktestExecutionConfig", () => {
    const display = buildStrategyConfigDisplayFromCanonical({
      executionConfig: canonicalExecutionConfig,
      metricCatalogs: catalog,
      rule: canonicalRule,
    })

    expect(display.conditionRows).toEqual([
      {
        expression: "收盘价 > 价格 MA20",
        groupLabel: "条件组 1",
        id: "条件组 1-1",
        logicLabel: "组内起始",
      },
      {
        expression: "换手率 区间内 2 - 8",
        groupLabel: "条件组 1",
        id: "条件组 1-2",
        logicLabel: "AND",
      },
    ])
    expect(display.scoringRows).toEqual([
      {
        expression: "KDJ J < -10",
        id: "kdj-low",
        index: 1,
        score: 25,
      },
    ])
    expect(display.buildSummaryRows).toEqual([
      { label: "初始资金", value: "¥1000000.00" },
      { label: "每日候选", value: "Top 5" },
      { label: "候选口径", value: "Top N 是每日候选信号，不是目标持仓集合" },
      { label: "调仓规则", value: "仅空位调入；旧持仓由风控退出" },
      { label: "最大持仓", value: "8 只" },
      { label: "单票上限", value: "10%" },
      { label: "交易成本", value: "佣金 0.01% / 滑点 0.12%" },
      {
        label: "风控",
        value: "固定止损 8%，止盈 20%，时间止损 20 天，指标止损 价格 MA10",
      },
    ])
  })
})

describe("readRuleVersionSpec", () => {
  it("accepts the canonical RuleVersionSpec shape", () => {
    expect(readRuleVersionSpec(JSON.parse(JSON.stringify(canonicalRule)))).toEqual(
      canonicalRule
    )
  })

  it("does not treat display-only JSON as canonical strategy config", () => {
    expect(
      readRuleVersionSpec({
        strategy_config_display: {
          version: 1,
          condition_rows: [],
          scoring_rows: [],
          build_summary_rows: [],
        },
      })
    ).toBeNull()
  })
})

describe("buildStrategyConfigSourceContext", () => {
  it("derives period and benchmark labels from frontend templates", () => {
    expect(
      buildStrategyConfigSourceContext({
        benchmark_security_code: "000905.SH",
        live_start_date: "2026-07-02",
        source_end_date: "2026-07-01",
        source_period_key: "1y",
        source_start_date: "2025-07-01",
      })
    ).toEqual({
      benchmarkLabel: "中证500",
      benchmarkSecurityCode: "000905.SH",
      liveStartDate: "2026-07-02",
      periodKey: "1y",
      periodLabel: "近一年",
      sourceEndDate: "2026-07-01",
      sourceStartDate: "2025-07-01",
    })
  })
})
