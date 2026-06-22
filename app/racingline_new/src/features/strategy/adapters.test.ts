import { describe, expect, it } from "vitest"

import {
  StrategyRuleSpecError,
  buildMixedLogicFilterExpr,
  buildStrategyMetricCatalog,
  buildStrategySelectionRuleSpec,
} from "@/features/strategy/adapters"
import type {
  ConditionOperator,
  StrategyCondition,
  StrategyConditionGroup,
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
    allow_scoring: true,
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

function compare(metricName: string, op: Operator, right: unknown) {
  return {
    type: "compare",
    left: { type: "metric", name: metricName },
    op,
    right,
  }
}

function numberOperand(value: number) {
  return {
    type: "number",
    value,
  }
}
