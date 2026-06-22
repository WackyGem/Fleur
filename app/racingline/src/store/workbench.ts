import { create } from "zustand"

import type { FilterExpr, Operand, Operator, RuleVersionSpec } from "@/types/rearview"

export type RuleDraftState = {
  ruleSetName: string
  ruleSetDescription: string
  owner: string
  universeBase: string
  excludeSt: boolean
  excludeSuspend: boolean
  includeSecurityCodes: string
  excludeSecurityCodes: string
  filterMetric: string
  filterOperator: Operator
  filterValue: string
  scoringMetric: string
  scoringWeight: string
  clampMin: string
  clampMax: string
  outputMetrics: string
  topNDefault: string
  runStartDate: string
  runEndDate: string
  runTopN: string
}

type WorkbenchStore = {
  draft: RuleDraftState
  selectedTradeDateByRun: Record<string, string>
  visibleMetricColumns: string[]
  signalDetailOpen: boolean
  setDraft: (patch: Partial<RuleDraftState>) => void
  setSelectedTradeDate: (runId: string, tradeDate: string) => void
  setVisibleMetricColumns: (columns: string[]) => void
  setSignalDetailOpen: (open: boolean) => void
  resetDraft: () => void
}

export const defaultRuleDraft: RuleDraftState = {
  ruleSetName: "",
  ruleSetDescription: "",
  owner: "racingline",
  universeBase: "all_a_shares",
  excludeSt: true,
  excludeSuspend: true,
  includeSecurityCodes: "",
  excludeSecurityCodes: "",
  filterMetric: "",
  filterOperator: "gte",
  filterValue: "",
  scoringMetric: "",
  scoringWeight: "1",
  clampMin: "0",
  clampMax: "100",
  outputMetrics: "",
  topNDefault: "20",
  runStartDate: "",
  runEndDate: "",
  runTopN: "",
}

const lowReversalOutputMetrics = [
  "close_price_forward_adj",
  "kdj_j_value",
  "pct_amplitude",
  "pct_change",
  "volume",
  "prev_volume",
  "volume_ma_5",
  "price_ema2_10",
  "price_avg_ma_14_28_57_114",
  "price_avg_ma_3_6_12_24",
  "price_ma_20",
  "price_ma_60",
  "price_ma_114",
  "price_ma_250",
  "boll_dn_20_2",
  "rsi_6",
  "close_down_streak_days",
  "n_structure_20_second_low_ratio",
]

export const useWorkbenchStore = create<WorkbenchStore>()((set) => ({
  draft: defaultRuleDraft,
  selectedTradeDateByRun: {},
  visibleMetricColumns: [],
  signalDetailOpen: false,
  setDraft: (patch) =>
    set((state) => ({
      draft: {
        ...state.draft,
        ...patch,
      },
    })),
  setSelectedTradeDate: (runId, tradeDate) =>
    set((state) => ({
      selectedTradeDateByRun: {
        ...state.selectedTradeDateByRun,
        [runId]: tradeDate,
      },
    })),
  setVisibleMetricColumns: (columns) => set({ visibleMetricColumns: columns }),
  setSignalDetailOpen: (open) => set({ signalDetailOpen: open }),
  resetDraft: () => set({ draft: defaultRuleDraft }),
}))

export function buildRuleVersionSpec(draft: RuleDraftState): RuleVersionSpec {
  const filterMetric = draft.filterMetric.trim()
  const scoringMetric = draft.scoringMetric.trim() || filterMetric
  const filterValue = Number(draft.filterValue || 0)
  const scoringWeight = Number(draft.scoringWeight || 1)
  const topNDefault = Number(draft.topNDefault || 20)
  const clampMin = Number(draft.clampMin || 0)
  const clampMax = Number(draft.clampMax || 100)

  return {
    universe: {
      base: draft.universeBase.trim() || "stock_daily",
      exclude_st: draft.excludeSt,
      exclude_suspend: draft.excludeSuspend,
      include_security_codes: splitCsv(draft.includeSecurityCodes),
      exclude_security_codes: splitCsv(draft.excludeSecurityCodes),
    },
    pool_filters: {
      type: "all",
      conditions: [
        {
          type: "compare",
          left: { type: "metric", name: filterMetric },
          op: draft.filterOperator,
          right:
            draft.filterOperator === "is_null"
              ? null
              : { type: "number", value: filterValue },
        },
      ],
    },
    scoring: {
      rules: [
        {
          type: "weighted_metric",
          name: "primary_weighted_metric",
          metric: scoringMetric,
          weight: scoringWeight,
        },
      ],
      clamp: {
        min: clampMin,
        max: clampMax,
      },
    },
    top_n_default: topNDefault,
    output_metrics: splitCsv(draft.outputMetrics),
  }
}

export function buildLowReversalRuleVersionSpec(
  draft: RuleDraftState,
): RuleVersionSpec {
  const kdjJ = metric("kdj_j_value")
  const forwardClose = metric("close_price_forward_adj")

  return {
    universe: {
      base: draft.universeBase.trim() || "all_a_shares",
      exclude_st: draft.excludeSt,
      exclude_suspend: draft.excludeSuspend,
      include_security_codes: splitCsv(draft.includeSecurityCodes),
      exclude_security_codes: splitCsv(draft.excludeSecurityCodes),
    },
    pool_filters: all([
      compare(kdjJ, "lt", number(13)),
      compare(metric("pct_amplitude"), "lt", number(4)),
      compare(metric("pct_change"), "gt", number(-2)),
      compare(metric("pct_change"), "lt", number(2)),
      compare(
        metric("volume"),
        "lt",
        multiply(metric("prev_volume"), number(0.8)),
      ),
      compare(
        metric("price_ema2_10"),
        "gt",
        metric("price_avg_ma_14_28_57_114"),
      ),
      compare(forwardClose, "gt", metric("price_avg_ma_3_6_12_24")),
      compare(metric("price_ma_60"), "gt", metric("price_ma_114")),
      compare(metric("price_ma_114"), "gt", metric("price_ma_250")),
      compare(metric("close_down_streak_days"), "lt", number(4)),
    ]),
    scoring: {
      rules: [
        points("kdj_j_below_minus_15", compare(kdjJ, "lt", number(-15)), 25),
        points(
          "kdj_j_between_minus_15_and_minus_10",
          all([
            compare(kdjJ, "gte", number(-15)),
            compare(kdjJ, "lt", number(-10)),
          ]),
          15,
        ),
        points(
          "volume_dry_up",
          compare(
            metric("volume"),
            "lt",
            multiply(metric("volume_ma_5"), number(0.6)),
          ),
          20,
        ),
        points(
          "between_ma_20_and_ma_60",
          all([
            compare(metric("price_ma_20"), "lt", forwardClose),
            compare(forwardClose, "lt", metric("price_ma_60")),
          ]),
          15,
        ),
        points(
          "n_structure_20_second_low_ratio_above_1",
          compare(metric("n_structure_20_second_low_ratio"), "gt", number(1)),
          15,
        ),
        points(
          "below_boll_dn_20_2",
          compare(forwardClose, "lt", metric("boll_dn_20_2")),
          15,
        ),
        points("rsi_6_below_25", compare(metric("rsi_6"), "lt", number(25)), 5),
      ],
      clamp: {
        min: 0,
        max: 100,
      },
    },
    top_n_default: Number(draft.topNDefault || 10),
    output_metrics: lowReversalOutputMetrics,
  }
}

export function splitCsv(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean)
}

function all(conditions: FilterExpr[]): FilterExpr {
  return { type: "all", conditions }
}

function compare(
  left: Operand,
  op: Operator,
  right: Operand,
): FilterExpr {
  return { type: "compare", left, op, right }
}

function metric(name: string): Operand {
  return { type: "metric", name }
}

function number(value: number): Operand {
  return { type: "number", value }
}

function multiply(left: Operand, right: Operand): Operand {
  return { type: "binary", op: "multiply", left, right }
}

function points(
  name: string,
  condition: RuleVersionSpec["pool_filters"],
  pointValue: number,
) {
  return {
    type: "conditional_points",
    name,
    condition,
    points: pointValue,
  } as const
}
