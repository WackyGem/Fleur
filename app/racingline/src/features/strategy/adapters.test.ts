import { describe, expect, it } from "vitest"

import {
  StrategyRuleSpecError,
  buildMixedLogicFilterExpr,
  buildStrategyMetricCatalog,
  buildStrategyPreviewRuleSpec,
  buildStrategyScoringCatalog,
  buildStrategySelectionRuleSpec,
  buildStrategyWeightScoring,
} from "@/features/strategy/adapters"
import type {
  ConditionOperator,
  StrategyCondition,
  StrategyConditionGroup,
  WeightExtraCondition,
  WeightIndicator,
} from "@/features/strategy/types"
import type {
  MetricDefinition,
  MetricValueKind,
  Operator,
} from "@/types/rearview"

const numericOps: Operator[] = [
  "eq",
  "ne",
  "lt",
  "lte",
  "gt",
  "gte",
  "between",
  "is_null",
]
const crossingOps: Operator[] = [
  ...numericOps,
  "crosses_above",
  "crosses_below",
]

const catalog = [
  metric("close_price", {
    defaultOutput: true,
    displayGroup: "quotes",
    labelZh: "收盘价",
    sortOrder: 10,
  }),
  metric("kdj_j_value", {
    defaultOutput: true,
    displayGroup: "momentum",
    labelZh: "KDJ J",
    sortOrder: 10,
  }),
  metric("price_ma_5", {
    cross: "prev_price_ma_5",
    displayGroup: "trend",
    labelZh: "MA5",
    ops: crossingOps,
    sortOrder: 20,
  }),
  metric("price_ma_20", {
    cross: "prev_price_ma_20",
    displayGroup: "trend",
    labelZh: "MA20",
    ops: crossingOps,
    sortOrder: 70,
  }),
  metric("price_ma_60", {
    cross: "prev_price_ma_60",
    displayGroup: "trend",
    labelZh: "MA60",
    ops: crossingOps,
    sortOrder: 80,
  }),
  metric("prev_price_ma_5", {
    allowFilter: false,
    displayGroup: "trend_previous",
    labelZh: "前值 MA5",
    sortOrder: 20,
  }),
  metric("prev_price_ma_20", {
    allowFilter: false,
    displayGroup: "trend_previous",
    labelZh: "前值 MA20",
    sortOrder: 70,
  }),
  metric("n_structure_20_is_valid", {
    defaultOutput: true,
    displayGroup: "pattern",
    kind: "boolean",
    labelZh: "N 字结构有效",
    ops: ["eq", "ne", "is_null"],
    sortOrder: 20,
  }),
  metric("listed_date", {
    displayGroup: "quotes",
    kind: "date",
    labelZh: "上市日期",
    ops: ["eq"],
    sortOrder: 90,
  }),
  metric("scoring_disabled_metric", {
    allowScoring: false,
    displayGroup: "momentum",
    labelZh: "不可评分指标",
    sortOrder: 99,
  }),
]

describe("buildStrategyMetricCatalog", () => {
  it("uses Chinese indicator type labels while preserving stable group ids", () => {
    const uiCatalog = buildStrategyMetricCatalog(catalog)

    expect(uiCatalog.map((group) => [group.id, group.label])).toEqual([
      ["quotes", "行情与涨跌"],
      ["trend", "趋势指标"],
      ["momentum", "动量指标"],
      ["pattern", "形态特征"],
    ])
    expect(
      uiCatalog.flatMap((group) => group.metrics.map((item) => item.id))
    ).not.toContain("prev_price_ma_5")
    expect(
      uiCatalog.flatMap((group) => group.metrics.map((item) => item.id))
    ).not.toContain("listed_date")
  })

  it("shows crossing operators only when previous_metric exists", () => {
    const uiCatalog = buildStrategyMetricCatalog(catalog)
    const ma5 = uiCatalog
      .flatMap((group) => group.metrics)
      .find((item) => item.id === "price_ma_5")
    const kdj = uiCatalog
      .flatMap((group) => group.metrics)
      .find((item) => item.id === "kdj_j_value")

    expect(ma5?.label).toBe("MA5")
    expect(ma5?.allowedOps).toContain("crosses_above")
    expect(kdj?.allowedOps).not.toContain("crosses_above")
  })
})

describe("buildStrategyScoringCatalog", () => {
  it("includes only metrics that are allowed for scoring", () => {
    const uiCatalog = buildStrategyScoringCatalog(catalog)
    const metricIds = uiCatalog.flatMap((group) =>
      group.metrics.map((item) => item.id)
    )

    expect(metricIds).toContain("kdj_j_value")
    expect(metricIds).toContain("prev_price_ma_5")
    expect(metricIds).not.toContain("scoring_disabled_metric")
  })
})

describe("buildStrategyWeightScoring", () => {
  it("builds conditional scoring rules with a fixed 0-100 clamp", () => {
    const result = buildStrategyWeightScoring(
      [weight("w1", "kdj_j_value", "gte", "50", 60)],
      catalog
    )

    expect(result.scoring).toEqual({
      rules: [
        {
          type: "conditional_points",
          name: "weight:w1:1",
          condition: compare("kdj_j_value", "gte", numberOperand(50)),
          points: 60,
        },
      ],
      clamp: { min: 0, max: 100 },
    })
    expect(result.weightPaths).toEqual([
      { weightId: "w1", path: "scoring.rules.0.condition" },
    ])
  })

  it("scales total scoring points down to 100", () => {
    const result = buildStrategyWeightScoring(
      [
        weight("w1", "kdj_j_value", "gte", "50", 80),
        weight("w2", "close_price", "gte", "1", 40),
      ],
      catalog
    )

    expect(result.scoring.rules).toMatchObject([
      { name: "weight:w1:1", points: 66.6667 },
      { name: "weight:w2:2", points: 33.3333 },
    ])
  })

  it("rejects empty or zero scoring totals before preview", () => {
    expect(() => buildStrategyWeightScoring([], catalog)).toThrow(
      StrategyRuleSpecError
    )
    expect(() =>
      buildStrategyWeightScoring(
        [weight("w1", "kdj_j_value", "gte", "50", 0)],
        catalog
      )
    ).toThrow(StrategyRuleSpecError)
  })

  it("requires metric operands to be allowed for scoring", () => {
    expect(() =>
      buildStrategyWeightScoring(
        [
          weight("w1", "kdj_j_value", "gte", "0", 50, {
            compareMetric: "scoring_disabled_metric",
            target: "metric",
          }),
        ],
        catalog
      )
    ).toThrow(StrategyRuleSpecError)
  })

  it("builds metric comparison multipliers for scoring", () => {
    const result = buildStrategyWeightScoring(
      [
        weight("w1", "price_ma_5", "lt", "0", 50, {
          compareMetric: "price_ma_20",
          compareMultiplier: "0.6",
          target: "metric",
        }),
      ],
      catalog
    )

    expect(result.scoring.rules[0]).toMatchObject({
      condition: compare("price_ma_5", "lt", multiplyOperand("price_ma_20", 0.6)),
    })
  })

  it("builds mutually exclusive segmented scoring conditions", () => {
    const result = buildStrategyWeightScoring(
      [
        weight("w1", "kdj_j_value", "gte", "-15", 15, {
          extraConditions: [
            extraCondition("w1-extra", "kdj_j_value", "lt", "-10"),
          ],
        }),
      ],
      catalog
    )

    expect(result.scoring.rules[0]).toMatchObject({
      condition: all([
        compare("kdj_j_value", "gte", numberOperand(-15)),
        compare("kdj_j_value", "lt", numberOperand(-10)),
      ]),
    })
  })

  it("builds chained metric scoring comparisons", () => {
    const result = buildStrategyWeightScoring(
      [
        weight("w1", "close_price", "gt", "0", 15, {
          compareMetric: "price_ma_20",
          extraConditions: [
            extraCondition("w1-extra", "close_price", "lt", "0", {
              compareMetric: "price_ma_60",
              target: "metric",
            }),
          ],
          target: "metric",
        }),
      ],
      catalog
    )

    expect(result.scoring.rules[0]).toMatchObject({
      condition: all([
        compare("close_price", "gt", {
          type: "metric",
          name: "price_ma_20",
        }),
        compare("close_price", "lt", {
          type: "metric",
          name: "price_ma_60",
        }),
      ]),
    })
    expect(result.outputMetrics).toEqual(
      expect.arrayContaining(["close_price", "price_ma_20", "price_ma_60"])
    )
  })

  it("adds scoring metrics and crossing previous metrics to output metrics", () => {
    const result = buildStrategyWeightScoring(
      [
        weight("w1", "price_ma_5", "crosses_above", "0", 50, {
          compareMetric: "price_ma_20",
          target: "metric",
        }),
      ],
      catalog
    )

    expect(result.outputMetrics).toEqual([
      "prev_price_ma_20",
      "prev_price_ma_5",
      "price_ma_20",
      "price_ma_5",
    ])
  })
})

describe("buildStrategyPreviewRuleSpec", () => {
  it("combines Step 1 filters with Step 2 scoring and output metrics", () => {
    const result = buildStrategyPreviewRuleSpec(
      [group([condition("c1", "close_price", "gte", "1")])],
      [
        weight("w1", "price_ma_5", "crosses_above", "0", 50, {
          compareMetric: "price_ma_20",
          target: "metric",
        }),
      ],
      catalog,
      { topN: 12 }
    )

    expect(result.rule.pool_filters).toEqual({
      type: "all",
      conditions: [compare("close_price", "gte", numberOperand(1))],
    })
    expect(result.rule.scoring.rules).toHaveLength(1)
    expect(result.rule.top_n_default).toBe(12)
    expect(result.rule.output_metrics).toEqual(
      expect.arrayContaining([
        "close_price",
        "prev_price_ma_20",
        "prev_price_ma_5",
        "price_ma_20",
        "price_ma_5",
      ])
    )
    expect(result.weightPaths).toEqual([
      { weightId: "w1", path: "scoring.rules.0.condition" },
    ])
  })

  it("does not add catalog default outputs that the rule did not use", () => {
    const result = buildStrategyPreviewRuleSpec(
      [group([condition("c1", "close_price", "gte", "1")])],
      [weight("w1", "close_price", "gte", "1", 50)],
      catalog
    )

    expect(result.rule.output_metrics).toEqual(["close_price"])
  })
})

describe("buildStrategySelectionRuleSpec", () => {
  it("builds numeric comparisons", () => {
    const { rule } = buildStrategySelectionRuleSpec(
      [group([condition("a", "kdj_j_value", "gte", "0")])],
      catalog
    )

    expect(rule.pool_filters).toEqual({
      type: "all",
      conditions: [compare("kdj_j_value", "gte", numberOperand(0))],
    })
  })

  it("builds metric comparisons", () => {
    const { rule } = buildStrategySelectionRuleSpec(
      [
        group([
          condition("a", "price_ma_5", "gt", "0", {
            compareMetric: "price_ma_20",
            target: "metric",
          }),
        ]),
      ],
      catalog
    )

    expect(rule.pool_filters).toEqual({
      type: "all",
      conditions: [
        compare("price_ma_5", "gt", { type: "metric", name: "price_ma_20" }),
      ],
    })
  })

  it("builds metric comparison multipliers", () => {
    const { rule } = buildStrategySelectionRuleSpec(
      [
        group([
          condition("a", "price_ma_5", "lt", "0", {
            compareMetric: "price_ma_20",
            compareMultiplier: "0.8",
            target: "metric",
          }),
        ]),
      ],
      catalog
    )

    expect(rule.pool_filters).toEqual({
      type: "all",
      conditions: [
        compare("price_ma_5", "lt", multiplyOperand("price_ma_20", 0.8)),
      ],
    })
  })

  it("builds between range operands", () => {
    const { rule } = buildStrategySelectionRuleSpec(
      [
        group([
          condition("a", "kdj_j_value", "between", "10", {
            valueEnd: "20",
          }),
        ]),
      ],
      catalog
    )

    expect(rule.pool_filters).toEqual({
      type: "all",
      conditions: [
        compare("kdj_j_value", "between", {
          type: "range",
          min: numberOperand(10),
          max: numberOperand(20),
        }),
      ],
    })
  })

  it("builds boolean eq/ne and is_null without a right side", () => {
    const eqResult = buildStrategySelectionRuleSpec(
      [group([condition("a", "n_structure_20_is_valid", "eq", "true")])],
      catalog
    )
    const nullResult = buildStrategySelectionRuleSpec(
      [group([condition("a", "n_structure_20_is_valid", "is_null", "false")])],
      catalog
    )

    expect(eqResult.rule.pool_filters).toEqual({
      type: "all",
      conditions: [
        compare("n_structure_20_is_valid", "eq", {
          type: "bool",
          value: true,
        }),
      ],
    })
    expect(nullResult.rule.pool_filters).toEqual({
      type: "all",
      conditions: [
        {
          type: "compare",
          left: { type: "metric", name: "n_structure_20_is_valid" },
          op: "is_null",
        },
      ],
    })
  })

  it("builds crossing comparisons and includes previous metrics in output", () => {
    const metricCross = buildStrategySelectionRuleSpec(
      [
        group([
          condition("a", "price_ma_5", "crosses_above", "0", {
            compareMetric: "price_ma_20",
            target: "metric",
          }),
        ]),
      ],
      catalog
    )
    const constantCross = buildStrategySelectionRuleSpec(
      [group([condition("a", "price_ma_5", "crosses_below", "10")])],
      catalog
    )

    expect(metricCross.rule.pool_filters).toEqual({
      type: "all",
      conditions: [
        compare("price_ma_5", "crosses_above", {
          type: "metric",
          name: "price_ma_20",
        }),
      ],
    })
    expect(metricCross.rule.output_metrics).toEqual(
      expect.arrayContaining(["prev_price_ma_5", "prev_price_ma_20"])
    )
    expect(constantCross.rule.pool_filters).toEqual({
      type: "all",
      conditions: [compare("price_ma_5", "crosses_below", numberOperand(10))],
    })
  })

  it("rejects empty condition groups before explain", () => {
    expect(() => buildStrategySelectionRuleSpec([group([])], catalog)).toThrow(
      StrategyRuleSpecError
    )
  })
})

describe("buildMixedLogicFilterExpr", () => {
  it("builds A and B or C as any([all([A, B]), C])", () => {
    const expr = buildMixedLogicFilterExpr(
      [
        condition("a", "close_price", "gte", "1", { logic: "or" }),
        condition("b", "price_ma_5", "gte", "1", { logic: "and" }),
        condition("c", "kdj_j_value", "gte", "1", { logic: "or" }),
      ],
      catalog
    )

    expect(expr).toEqual({
      type: "any",
      conditions: [
        {
          type: "all",
          conditions: [
            compare("close_price", "gte", numberOperand(1)),
            compare("price_ma_5", "gte", numberOperand(1)),
          ],
        },
        compare("kdj_j_value", "gte", numberOperand(1)),
      ],
    })
  })

  it("builds A or B and C as any([A, all([B, C])])", () => {
    const expr = buildMixedLogicFilterExpr(
      [
        condition("a", "close_price", "gte", "1"),
        condition("b", "price_ma_5", "gte", "1", { logic: "or" }),
        condition("c", "kdj_j_value", "gte", "1", { logic: "and" }),
      ],
      catalog
    )

    expect(expr).toEqual({
      type: "any",
      conditions: [
        compare("close_price", "gte", numberOperand(1)),
        {
          type: "all",
          conditions: [
            compare("price_ma_5", "gte", numberOperand(1)),
            compare("kdj_j_value", "gte", numberOperand(1)),
          ],
        },
      ],
    })
  })

  it("builds A and B or C and D as two AND segments under any", () => {
    const expr = buildMixedLogicFilterExpr(
      [
        condition("a", "close_price", "gte", "1"),
        condition("b", "price_ma_5", "gte", "1", { logic: "and" }),
        condition("c", "kdj_j_value", "gte", "1", { logic: "or" }),
        condition("d", "price_ma_20", "gte", "1", { logic: "and" }),
      ],
      catalog
    )

    expect(expr).toEqual({
      type: "any",
      conditions: [
        {
          type: "all",
          conditions: [
            compare("close_price", "gte", numberOperand(1)),
            compare("price_ma_5", "gte", numberOperand(1)),
          ],
        },
        {
          type: "all",
          conditions: [
            compare("kdj_j_value", "gte", numberOperand(1)),
            compare("price_ma_20", "gte", numberOperand(1)),
          ],
        },
      ],
    })
  })

  it("builds multiple groups under a top-level all", () => {
    const { rule } = buildStrategySelectionRuleSpec(
      [
        group([
          condition("a", "close_price", "gte", "1"),
          condition("b", "price_ma_5", "gte", "1", { logic: "and" }),
          condition("c", "kdj_j_value", "gte", "1", { logic: "or" }),
        ]),
        group([
          condition("d", "price_ma_20", "gte", "1"),
          condition("e", "kdj_j_value", "gte", "1", { logic: "or" }),
          condition("f", "price_ma_5", "gte", "1", { logic: "and" }),
        ]),
      ],
      catalog
    )

    expect(rule.pool_filters).toEqual({
      type: "all",
      conditions: [
        {
          type: "any",
          conditions: [
            {
              type: "all",
              conditions: [
                compare("close_price", "gte", numberOperand(1)),
                compare("price_ma_5", "gte", numberOperand(1)),
              ],
            },
            compare("kdj_j_value", "gte", numberOperand(1)),
          ],
        },
        {
          type: "any",
          conditions: [
            compare("price_ma_20", "gte", numberOperand(1)),
            {
              type: "all",
              conditions: [
                compare("kdj_j_value", "gte", numberOperand(1)),
                compare("price_ma_5", "gte", numberOperand(1)),
              ],
            },
          ],
        },
      ],
    })
  })
})

function metric(
  name: string,
  options: {
    allowFilter?: boolean
    allowScoring?: boolean
    cross?: string
    defaultOutput?: boolean
    displayGroup?: string
    kind?: MetricValueKind
    labelZh?: string
    ops?: Operator[]
    sortOrder?: number
  } = {}
): MetricDefinition {
  return {
    logical_metric: name,
    mart_database: "fleur_marts",
    mart_table: martTableForGroup(options.displayGroup ?? "momentum"),
    column_name: name,
    value_kind: options.kind ?? "numeric",
    allow_filter: options.allowFilter ?? true,
    allow_scoring: options.allowScoring ?? true,
    allowed_ops: options.ops ?? numericOps,
    null_policy: "no_match",
    default_output: options.defaultOutput ?? false,
    description: `${name} description`,
    cross: options.cross ? { previous_metric: options.cross } : null,
    display: {
      group: options.displayGroup ?? "momentum",
      label_zh: options.labelZh ?? name,
      sort_order: options.sortOrder ?? 10,
    },
  }
}

function martTableForGroup(groupName: string) {
  const tables: Record<string, string> = {
    momentum: "mart_stock_momentum_indicator_daily",
    pattern: "mart_stock_price_pattern_daily",
    quotes: "mart_stock_quotes_daily",
    trend: "mart_stock_trend_indicator_daily",
    trend_previous: "mart_stock_trend_indicator_daily",
  }

  return tables[groupName] ?? "mart_stock_momentum_indicator_daily"
}

function group(conditions: StrategyCondition[]): StrategyConditionGroup {
  return {
    id: "group",
    name: "指标组",
    conditions,
  }
}

function condition(
  id: string,
  metricName: string,
  operator: ConditionOperator,
  value: string,
  overrides: Partial<StrategyCondition> = {}
): StrategyCondition {
  return {
    id,
    catalogId: "test",
    metric: metricName,
    target: "value",
    operator,
    value,
    valueEnd: "",
    compareCatalogId: "test",
    compareMetric: "close_price",
    logic: "and",
    ...overrides,
  }
}

function weight(
  id: string,
  metricName: string,
  operator: ConditionOperator,
  value: string,
  score: number,
  overrides: Partial<WeightIndicator> = {}
): WeightIndicator {
  return {
    ...condition(id, metricName, operator, value, overrides),
    id,
    score,
  }
}

function extraCondition(
  id: string,
  metricName: string,
  operator: ConditionOperator,
  value: string,
  overrides: Partial<WeightExtraCondition> = {}
): WeightExtraCondition {
  return {
    catalogId: "test",
    metric: metricName,
    target: "value",
    operator,
    value,
    valueEnd: "",
    compareCatalogId: "test",
    compareMetric: "close_price",
    id,
    ...overrides,
  }
}

function compare(metricName: string, op: Operator, right: unknown) {
  return {
    type: "compare",
    left: { type: "metric", name: metricName },
    op,
    right,
  }
}

function all(conditions: unknown[]) {
  return {
    type: "all",
    conditions,
  }
}

function numberOperand(value: number) {
  return {
    type: "number",
    value,
  }
}

function multiplyOperand(metricName: string, multiplier: number) {
  return {
    type: "binary",
    op: "multiply",
    left: { type: "metric", name: metricName },
    right: numberOperand(multiplier),
  }
}
