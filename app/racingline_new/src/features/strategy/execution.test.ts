import { describe, expect, it } from "vitest"

import {
  buildBacktestDateRange,
  buildBacktestExecutionRequestDraft,
  buildStrategyBacktestValidateRequest,
  marketTemplateToTransactionFees,
  simulationSettingsToBacktestExecutionConfig,
  toBacktestExecutionDraft,
} from "@/features/strategy/execution"
import type { PreviewSnapshot } from "@/features/strategy/preview"
import type { SimulationSettings } from "@/features/strategy/types"
import type {
  MarketFeeTemplateRecord,
  RuleVersionSpec,
  StrategyBacktestDraftResponse,
} from "@/types/rearview"

const rule: RuleVersionSpec = {
  output_metrics: ["close_price"],
  pool_filters: { conditions: [], type: "all" },
  scoring: {
    clamp: { max: 100, min: 0 },
    rules: [],
  },
  top_n_default: 20,
  universe: {
    base: "all_a_shares",
    exclude_security_codes: [],
    exclude_st: true,
    exclude_suspend: true,
    include_security_codes: [],
  },
}

const marketTemplate: MarketFeeTemplateRecord = {
  currency: "CNY",
  fee_profile: {
    commission_rate: 0.0001,
    commission_rate_max: 0.003,
    min_commission: 5,
    stamp_duty_rate_sell: 0.0005,
    transfer_fee_rate: 0.00001,
  },
  is_default: true,
  market: "CN_A_SHARE",
  market_fee_template_id: "template-1",
  name: "CN A Share default",
  slippage_profile: {
    mode: "bps",
    buy_bps: 10,
    sell_bps: 12,
  },
  status: "active",
}

const settings: SimulationSettings = {
  initialCapital: 1_000_000,
  buyTopN: 5,
  singlePositionLimitPercent: 10,
  transactionFees: {
    commissionRatePercent: 0.01,
    stampDutyRatePercent: 0.05,
    transferFeeRatePercent: 0.001,
    slippageRatePercent: 0.12,
  },
  fixedStopLoss: {
    enabled: true,
    lossPercent: 8,
  },
  indicatorStopLoss: {
    enabled: false,
    catalogId: "trend",
    metric: "price_ma_10",
  },
  takeProfit: {
    enabled: true,
    profitPercent: 20,
  },
  timeStopLoss: {
    enabled: true,
    holdingDays: 20,
    minimumReturnPercent: 0,
  },
}

describe("marketTemplateToTransactionFees", () => {
  it("converts template decimals and bps into UI percentages", () => {
    expect(marketTemplateToTransactionFees(marketTemplate)).toEqual({
      commissionRatePercent: 0.01,
      stampDutyRatePercent: 0.05,
      transferFeeRatePercent: 0.001,
      slippageRatePercent: 0.12,
    })
  })
})

describe("simulationSettingsToBacktestExecutionConfig", () => {
  it("maps Step4 fields into the Rearview snake_case execution config", () => {
    const config = simulationSettingsToBacktestExecutionConfig(
      settings,
      marketTemplate
    )

    expect(config.signal_policy.buy_signal_top_n).toBe(5)
    expect(config.rebalance_policy.max_positions).toBe(5)
    expect(config.rebalance_policy.single_position_limit_pct).toBe(0.1)
    expect(config.fee_profile.commission_rate).toBe(0.0001)
    expect(config.fee_profile.commission_rate_max).toBe(0.003)
    expect(config.fee_profile.min_commission).toBe(5)
    expect(config.fee_profile.stamp_duty_rate_sell).toBe(0.0005)
    expect(config.fee_profile.transfer_fee_rate).toBe(0.00001)
    expect(config.slippage_profile.buy_bps).toBe(12)
    expect(config.slippage_profile.sell_bps).toBe(12)
    expect(config.risk_exit_policy.exit_rules).toEqual([
      { type: "fixed_stop_loss", loss_pct: 0.08 },
      { type: "take_profit", profit_pct: 0.2 },
      { type: "time_stop_loss", holding_days: 20, max_return_pct: 0 },
    ])
  })

  it("serializes trend indicator stop loss", () => {
    const config = simulationSettingsToBacktestExecutionConfig(
      {
        ...settings,
        indicatorStopLoss: {
          ...settings.indicatorStopLoss,
          enabled: true,
        },
      },
      marketTemplate
    )

    expect(config.risk_exit_policy.exit_rules.at(-1)).toEqual({
      type: "indicator_stop_loss",
      source: "trend",
      metric: "price_ma_10",
      operator: "close_below_metric",
    })
  })

  it("rejects commission rates above the template max", () => {
    expect(() =>
      simulationSettingsToBacktestExecutionConfig(
        {
          ...settings,
          transactionFees: {
            ...settings.transactionFees,
            commissionRatePercent: 0.31,
          },
        },
        marketTemplate
      )
    ).toThrow("佣金率")
  })
})

describe("buildStrategyBacktestValidateRequest", () => {
  it("uses a non-stale preview snapshot as the validate request boundary", () => {
    const request = buildStrategyBacktestValidateRequest({
      marketTemplate,
      previewSnapshot: previewSnapshot(false),
      settings,
    })

    expect(request.rule).toBe(rule)
    expect(request.preview_id).toBe("preview-1")
    expect(request.preview_range).toEqual({
      start_date: "2025-06-22",
      end_date: "2026-06-22",
    })
  })

  it("rejects stale preview snapshots", () => {
    expect(() =>
      buildStrategyBacktestValidateRequest({
        marketTemplate,
        previewSnapshot: previewSnapshot(true),
        settings,
      })
    ).toThrow("股池预览已过期")
  })
})

describe("buildBacktestExecutionRequestDraft", () => {
  it("combines the backend draft with Step5 range and benchmark", () => {
    const request = buildStrategyBacktestValidateRequest({
      marketTemplate,
      previewSnapshot: previewSnapshot(false),
      settings,
    })
    const response: StrategyBacktestDraftResponse = {
      execution_config: request.execution_config,
      execution_config_hash: "execution-hash",
      preview_id: "preview-1",
      preview_range: request.preview_range,
      rule_hash: "rule-hash",
      summary: {
        buy_signal_top_n: 5,
        enabled_exit_rule_count: 3,
        implicit_cash_reserve_pct: 0.5,
        max_positions: 5,
        target_weight_per_position_pct: 0.1,
      },
      warnings: [],
    }
    const draft = toBacktestExecutionDraft({
      createdAt: "2026-06-22T00:00:00.000Z",
      request,
      response,
    })

    expect(
      buildBacktestExecutionRequestDraft({
        benchmark: "000300.SH",
        draft,
        now: new Date("2026-06-22T12:00:00Z"),
        period: "2y",
      })
    ).toMatchObject({
      benchmark: "000300.SH",
      end_date: "2026-06-22",
      execution_config_hash: "execution-hash",
      rule_hash: "rule-hash",
      start_date: "2024-06-22",
      top_n: 5,
    })
  })
})

describe("buildBacktestDateRange", () => {
  it("builds UTC date ranges for period presets", () => {
    expect(
      buildBacktestDateRange("1y", new Date("2026-06-22T23:59:00Z"))
    ).toEqual({
      start_date: "2025-06-22",
      end_date: "2026-06-22",
    })
  })
})

function previewSnapshot(stale: boolean): PreviewSnapshot {
  return {
    appliedRuleSpec: rule,
    createdAt: "2026-06-22T00:00:00.000Z",
    labels: {
      filterMetrics: [],
      metrics: {},
      scoringRules: {},
    },
    previewId: "preview-1",
    range: {
      endDate: "2026-06-22",
      previewRowLimit: 10,
      selectedTradeDate: "2026-06-22",
      startDate: "2025-06-22",
    },
    result: {
      end_date: "2026-06-22",
      preview_id: "preview-1",
      preview_row_limit: 10,
      required_columns: {},
      required_marts: [],
      required_metrics: ["close_price"],
      sql_hash: "hash",
      start_date: "2026-06-22",
      top_n: 10,
      trade_dates: [],
    },
    stale,
    timeline: null,
  }
}
