import { describe, expect, it } from "vitest"

import {
  buildPreviewPresentation,
  buildPreviewSnapshot,
  buildPreviewTimelineRange,
  markPreviewSnapshotStale,
} from "@/features/strategy/preview"
import type { MetricDefinition, RuleVersionSpec } from "@/types/rearview"

const metrics: MetricDefinition[] = [
  {
    allow_filter: true,
    allow_scoring: true,
    allowed_ops: ["gte"],
    column_name: "close_price",
    cross: null,
    default_output: true,
    description: null,
    display: {
      group: "quotes",
      label_zh: "收盘价",
      sort_order: 1,
      unit: "元",
    },
    logical_metric: "close_price",
    mart_database: "fleur_marts",
    mart_table: "mart_stock_quotes_daily",
    null_policy: "no_match",
    value_kind: "numeric",
  },
]

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

describe("buildPreviewPresentation", () => {
  it("uses snapshot labels and keeps selected metrics and raw values", () => {
    const snapshot = buildPreviewSnapshot({
      appliedRuleSpec: rule,
      conditionGroups: [
        {
          conditions: [
            {
              catalogId: "quotes",
              compareCatalogId: "quotes",
              compareMetric: "close_price",
              id: "c1",
              logic: "and",
              metric: "close_price",
              operator: "gte",
              target: "value",
              value: "10",
              valueEnd: "",
            },
          ],
          id: "g1",
          name: "指标组 1",
        },
      ],
      conditionPaths: [
        {
          conditionId: "c1",
          groupId: "g1",
          path: "pool_filters.conditions.0",
        },
      ],
      createdAt: "2026-06-22T00:00:00.000Z",
      metrics,
      range: {
        endDate: "2026-06-02",
        previewRowLimit: 10,
        startDate: "2026-06-02",
      },
      result: {
        end_date: "2026-06-02",
        preview_id: "preview-1",
        preview_row_limit: 10,
        required_columns: {},
        required_marts: [],
        required_metrics: ["close_price"],
        sql_hash: "hash",
        start_date: "2026-06-02",
        top_n: 10,
        trade_dates: [
          {
            pool_count: 12,
            signals: [
              {
                exchange_code: "SH",
                is_buy_signal: true,
                raw_score: 88,
                raw_values: { close_price: 10.25, unknown_metric: 7 },
                score: 88,
                score_breakdown: { "weight:w1:1": 88 },
                security_code: "600000.SH",
                security_board: "sse_main_board",
                security_name: "浦发银行",
                selected_metrics: { close_price: 10.25 },
                signal_rank: 1,
              },
            ],
            trade_date: "2026-06-02",
          },
        ],
      },
      timeline: {
        end_date: "2026-06-02",
        preview_id: "timeline-1",
        required_columns: {},
        required_marts: [],
        required_metrics: ["close_price"],
        sql_hash: "timeline-hash",
        start_date: "2025-06-02",
        trade_dates: [
          {
            pool_count: 12,
            trade_date: "2026-06-02",
          },
        ],
      },
      weightIndicators: [
        {
          catalogId: "quotes",
          compareCatalogId: "quotes",
          compareMetric: "close_price",
          id: "w1",
          metric: "close_price",
          operator: "gte",
          score: 88,
          target: "value",
          value: "10",
          valueEnd: "",
        },
      ],
    })

    const presentation = buildPreviewPresentation(snapshot)

    expect(presentation.tradeDates[0]?.stocks[0]).toMatchObject({
      code: "600000.SH",
      board: "sse_main_board",
      boardLabel: "沪市主板",
      exchangeCode: "SH",
      name: "浦发银行",
      rawValueRows: [
        { id: "close_price", label: "收盘价", value: "10.25" },
        { id: "unknown_metric", label: "unknown_metric", value: "7" },
      ],
      filterMetricRows: [{ id: "c1", label: "收盘价", value: "10.25" }],
      scoreItems: [{ id: "weight:w1:1", score: 88 }],
    })
  })

  it("uses timeline dates even when preview rows only cover one date", () => {
    const snapshot = buildPreviewSnapshot({
      appliedRuleSpec: rule,
      conditionGroups: [],
      conditionPaths: [],
      createdAt: "2026-06-22T00:00:00.000Z",
      metrics,
      range: {
        endDate: "2026-06-02",
        previewRowLimit: 10,
        startDate: "2025-06-02",
      },
      result: {
        end_date: "2026-06-02",
        preview_id: "preview-1",
        preview_row_limit: 10,
        required_columns: {},
        required_marts: [],
        required_metrics: ["close_price"],
        sql_hash: "hash",
        start_date: "2026-06-02",
        top_n: 10,
        trade_dates: [],
      },
      timeline: {
        end_date: "2026-06-02",
        preview_id: "timeline-1",
        required_columns: {},
        required_marts: [],
        required_metrics: ["close_price"],
        sql_hash: "timeline-hash",
        start_date: "2025-06-02",
        trade_dates: [
          { pool_count: 5, trade_date: "2026-06-01" },
          { pool_count: 8, trade_date: "2026-06-02" },
        ],
      },
      weightIndicators: [],
    })

    const presentation = buildPreviewPresentation(snapshot)

    expect(presentation.tradeDates.map((row) => row.date)).toEqual([
      "2026-06-01",
      "2026-06-02",
    ])
  })
})

describe("markPreviewSnapshotStale", () => {
  it("keeps null snapshots unchanged", () => {
    expect(markPreviewSnapshotStale(null)).toBeNull()
  })
})

describe("buildPreviewTimelineRange", () => {
  it("builds a near-one-year range from the current date", () => {
    const range = buildPreviewTimelineRange(new Date(2026, 5, 22), 10)

    expect(range).toEqual({
      endDate: "2026-06-22",
      previewRowLimit: 10,
      startDate: "2025-06-22",
    })
  })
})
