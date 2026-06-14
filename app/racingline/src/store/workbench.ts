import { create } from "zustand"

import type { Operator, RuleVersionSpec } from "@/types/rearview"

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
  universeBase: "stock_daily",
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
  clampMax: "99",
  outputMetrics: "",
  topNDefault: "20",
  runStartDate: "",
  runEndDate: "",
  runTopN: "",
}

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
  const clampMax = Number(draft.clampMax || 99)

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

export function splitCsv(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean)
}
