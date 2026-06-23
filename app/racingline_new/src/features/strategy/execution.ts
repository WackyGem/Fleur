import type { PreviewSnapshot } from "@/features/strategy/preview"
import type { SimulationSettings } from "@/features/strategy/types"
import type {
  BacktestDateRange,
  BacktestExecutionConfig,
  MarketFeeTemplateRecord,
  RuleVersionSpec,
  StrategyBacktestDraftResponse,
  StrategyBacktestValidateRequest,
} from "@/types/rearview"

export type BacktestPeriodValue = "1y" | "2y" | "3y"

export type BacktestExecutionDraft = StrategyBacktestDraftResponse & {
  appliedRuleSpec: RuleVersionSpec
  createdAt: string
  stale: boolean
}

export type BacktestExecutionRequestDraft = {
  rule: RuleVersionSpec
  start_date: string
  end_date: string
  benchmark: string
  top_n: number
  execution_config: BacktestExecutionConfig
  rule_hash: string
  execution_config_hash: string
}

export class StrategyBacktestExecutionError extends Error {
  constructor(message: string) {
    super(message)
    this.name = "StrategyBacktestExecutionError"
  }
}

export function marketTemplateToTransactionFees(
  template: MarketFeeTemplateRecord
): SimulationSettings["transactionFees"] {
  return {
    commissionRatePercent: decimalToPercent(
      template.fee_profile.commission_rate
    ),
    commissionRateMaxPercent: decimalToPercent(
      template.fee_profile.commission_rate_max
    ),
    minCommission: template.fee_profile.min_commission,
    stampDutyRatePercent: decimalToPercent(
      template.fee_profile.stamp_duty_rate_sell
    ),
    transferFeeRatePercent: decimalToPercent(
      template.fee_profile.transfer_fee_rate
    ),
    buySlippageRatePercent: bpsToPercent(template.slippage_profile.buy_bps),
    sellSlippageRatePercent: bpsToPercent(template.slippage_profile.sell_bps),
  }
}

export function areTransactionFeesEqual(
  left: SimulationSettings["transactionFees"],
  right: SimulationSettings["transactionFees"]
) {
  return (
    left.commissionRatePercent === right.commissionRatePercent &&
    left.commissionRateMaxPercent === right.commissionRateMaxPercent &&
    left.minCommission === right.minCommission &&
    left.stampDutyRatePercent === right.stampDutyRatePercent &&
    left.transferFeeRatePercent === right.transferFeeRatePercent &&
    left.buySlippageRatePercent === right.buySlippageRatePercent &&
    left.sellSlippageRatePercent === right.sellSlippageRatePercent
  )
}

export function simulationSettingsToBacktestExecutionConfig(
  settings: SimulationSettings,
  marketTemplate: MarketFeeTemplateRecord
): BacktestExecutionConfig {
  if (marketTemplate.market !== "CN_A_SHARE") {
    throw new StrategyBacktestExecutionError(
      "模拟建仓第一版只支持 CN_A_SHARE 市场模板"
    )
  }
  if (marketTemplate.currency !== "CNY") {
    throw new StrategyBacktestExecutionError("模拟建仓第一版只支持 CNY 账户")
  }
  if (settings.indicatorStopLoss.enabled) {
    throw new StrategyBacktestExecutionError(
      "指标止损尚未接入 Rearview portfolio engine，不能进入回测草稿"
    )
  }

  const buyTopN = Math.max(1, Math.floor(settings.buyTopN))

  return {
    market: "CN_A_SHARE",
    account: {
      initial_cash: settings.initialCapital,
      currency: "CNY",
    },
    signal_policy: {
      buy_signal_top_n: buyTopN,
      signal_timing: "close_confirm_next_open",
    },
    rebalance_policy: {
      target_weighting: "equal_weight_capped",
      max_positions: buyTopN,
      single_position_limit_pct: percentToDecimal(
        settings.singlePositionLimitPercent
      ),
      cash_reserve_pct: 0,
      lot_size: 100,
      min_trade_lots: 1,
      empty_signal_action: "hold",
    },
    fee_profile: {
      commission_rate: percentToDecimal(
        settings.transactionFees.commissionRatePercent
      ),
      commission_rate_max: percentToDecimal(
        settings.transactionFees.commissionRateMaxPercent
      ),
      min_commission: settings.transactionFees.minCommission,
      stamp_duty_rate_sell: percentToDecimal(
        settings.transactionFees.stampDutyRatePercent
      ),
      transfer_fee_rate: percentToDecimal(
        settings.transactionFees.transferFeeRatePercent
      ),
    },
    slippage_profile: {
      mode: "bps",
      buy_bps: percentToBps(settings.transactionFees.buySlippageRatePercent),
      sell_bps: percentToBps(settings.transactionFees.sellSlippageRatePercent),
    },
    risk_exit_policy: {
      trigger_timing: "close_confirm_next_open",
      exit_rules: buildExitRules(settings),
    },
    price_basis: "backward_adjusted",
  }
}

export function buildStrategyBacktestValidateRequest({
  marketTemplate,
  previewSnapshot,
  settings,
}: {
  marketTemplate: MarketFeeTemplateRecord
  previewSnapshot: PreviewSnapshot
  settings: SimulationSettings
}): StrategyBacktestValidateRequest {
  if (previewSnapshot.stale) {
    throw new StrategyBacktestExecutionError(
      "股池预览已过期，需要先更新股池再生成回测草稿"
    )
  }

  return {
    rule: previewSnapshot.appliedRuleSpec,
    preview_id: previewSnapshot.previewId,
    preview_range: {
      start_date: previewSnapshot.range.startDate,
      end_date: previewSnapshot.range.endDate,
    },
    execution_config: simulationSettingsToBacktestExecutionConfig(
      settings,
      marketTemplate
    ),
  }
}

export function toBacktestExecutionDraft({
  createdAt,
  request,
  response,
}: {
  createdAt: string
  request: StrategyBacktestValidateRequest
  response: StrategyBacktestDraftResponse
}): BacktestExecutionDraft {
  return {
    ...response,
    appliedRuleSpec: request.rule,
    createdAt,
    stale: false,
  }
}

export function buildBacktestExecutionRequestDraft({
  benchmark,
  draft,
  now,
  period,
}: {
  benchmark: string
  draft: BacktestExecutionDraft
  now?: Date
  period: BacktestPeriodValue
}): BacktestExecutionRequestDraft {
  const range = buildBacktestDateRange(period, now)

  return {
    rule: draft.appliedRuleSpec,
    start_date: range.start_date,
    end_date: range.end_date,
    benchmark,
    top_n: draft.execution_config.signal_policy.buy_signal_top_n,
    execution_config: draft.execution_config,
    rule_hash: draft.rule_hash,
    execution_config_hash: draft.execution_config_hash,
  }
}

export function buildBacktestDateRange(
  period: BacktestPeriodValue,
  now = new Date()
): BacktestDateRange {
  const yearsByPeriod: Record<BacktestPeriodValue, number> = {
    "1y": 1,
    "2y": 2,
    "3y": 3,
  }
  const end = new Date(
    Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate())
  )
  const start = new Date(end)
  start.setUTCFullYear(start.getUTCFullYear() - yearsByPeriod[period])

  return {
    start_date: formatIsoDate(start),
    end_date: formatIsoDate(end),
  }
}

function buildExitRules(
  settings: SimulationSettings
): BacktestExecutionConfig["risk_exit_policy"]["exit_rules"] {
  const rules: BacktestExecutionConfig["risk_exit_policy"]["exit_rules"] = []

  if (settings.fixedStopLoss.enabled) {
    rules.push({
      type: "fixed_stop_loss",
      loss_pct: percentToDecimal(settings.fixedStopLoss.lossPercent),
    })
  }
  if (settings.takeProfit.enabled) {
    rules.push({
      type: "take_profit",
      profit_pct: percentToDecimal(settings.takeProfit.profitPercent),
    })
  }
  if (settings.timeStopLoss.enabled) {
    rules.push({
      type: "time_stop_loss",
      holding_days: Math.max(1, Math.floor(settings.timeStopLoss.holdingDays)),
      max_return_pct: percentToDecimal(
        settings.timeStopLoss.minimumReturnPercent
      ),
    })
  }

  return rules
}

function percentToDecimal(value: number) {
  return roundNumber(value / 100)
}

function percentToBps(value: number) {
  return roundNumber(value * 100)
}

function decimalToPercent(value: number) {
  return roundNumber(value * 100)
}

function bpsToPercent(value: number) {
  return roundNumber(value / 100)
}

function roundNumber(value: number) {
  return Number(value.toFixed(10))
}

function formatIsoDate(date: Date) {
  const year = date.getUTCFullYear()
  const month = String(date.getUTCMonth() + 1).padStart(2, "0")
  const day = String(date.getUTCDate()).padStart(2, "0")

  return `${year}-${month}-${day}`
}
