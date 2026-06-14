import { describe, expect, it } from "vitest"

import {
  buildRuleVersionSpec,
  defaultRuleDraft,
  splitCsv,
} from "@/store/workbench"

describe("workbench draft helpers", () => {
  it("splits comma separated fields", () => {
    expect(splitCsv(" a, ,b,c ")).toEqual(["a", "b", "c"])
  })

  it("builds a RuleVersionSpec from the structured draft", () => {
    const spec = buildRuleVersionSpec({
      ...defaultRuleDraft,
      excludeSecurityCodes: "000001.SZ",
      filterMetric: "kdj_j",
      filterOperator: "gte",
      filterValue: "80",
      includeSecurityCodes: "600000.SH, 000002.SZ",
      outputMetrics: "close, kdj_j",
      scoringMetric: "kdj_j",
      scoringWeight: "1.5",
      topNDefault: "10",
    })

    expect(spec.universe.include_security_codes).toEqual([
      "600000.SH",
      "000002.SZ",
    ])
    expect(spec.universe.exclude_security_codes).toEqual(["000001.SZ"])
    expect(spec.pool_filters).toEqual({
      conditions: [
        {
          left: { name: "kdj_j", type: "metric" },
          op: "gte",
          right: { type: "number", value: 80 },
          type: "compare",
        },
      ],
      type: "all",
    })
    expect(spec.scoring.rules).toEqual([
      {
        metric: "kdj_j",
        name: "primary_weighted_metric",
        type: "weighted_metric",
        weight: 1.5,
      },
    ])
    expect(spec.top_n_default).toBe(10)
    expect(spec.output_metrics).toEqual(["close", "kdj_j"])
  })

  it("omits right operand for is_null filters", () => {
    const spec = buildRuleVersionSpec({
      ...defaultRuleDraft,
      filterMetric: "suspension_flag",
      filterOperator: "is_null",
    })

    expect(spec.pool_filters).toEqual({
      conditions: [
        {
          left: { name: "suspension_flag", type: "metric" },
          op: "is_null",
          right: null,
          type: "compare",
        },
      ],
      type: "all",
    })
  })
})
