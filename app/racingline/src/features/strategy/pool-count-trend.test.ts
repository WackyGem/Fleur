import { describe, expect, it } from "vitest"

import { buildPoolCountTrendData } from "@/features/strategy/pool-count-trend"
import type { PreviewSnapshot } from "@/features/strategy/preview"
import type { RuleVersionSpec } from "@/types/rearview"

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

describe("buildPoolCountTrendData", () => {
  it("uses recent Step3 timeline pool_count values", () => {
    const data = buildPoolCountTrendData(previewSnapshot())

    expect(data).toEqual([
      { count: 8, label: "4/1" },
      { count: 13, label: "5/1" },
      { count: 21, label: "6/23" },
    ])
  })

  it("prefers Step3 timeline over preview result rows", () => {
    const snapshot = previewSnapshot()
    snapshot.result.trade_dates = [
      {
        pool_count: 999,
        signals: [],
        trade_date: "2026-06-23",
      },
    ]

    expect(buildPoolCountTrendData(snapshot)).toEqual([
      { count: 8, label: "4/1" },
      { count: 13, label: "5/1" },
      { count: 21, label: "6/23" },
    ])
  })
})

function previewSnapshot(): PreviewSnapshot {
  return {
    appliedRuleSpec: rule,
    createdAt: "2026-06-23T00:00:00.000Z",
    labels: {
      filterMetrics: [],
      metrics: {},
      scoringRules: {},
    },
    previewId: "preview-1",
    range: {
      endDate: "2026-06-23",
      previewRowLimit: 10,
      selectedTradeDate: "2026-06-23",
      startDate: "2025-06-23",
    },
    result: {
      end_date: "2026-06-23",
      preview_id: "preview-1",
      preview_row_limit: 10,
      required_columns: {},
      required_marts: [],
      required_metrics: ["close_price"],
      sql_hash: "hash",
      start_date: "2026-06-23",
      top_n: 10,
      trade_dates: [],
    },
    stale: false,
    timeline: {
      end_date: "2026-06-23",
      preview_id: "preview-1",
      required_columns: {},
      required_marts: [],
      required_metrics: ["close_price"],
      sql_hash: "hash",
      start_date: "2026-01-01",
      trade_dates: [
        { pool_count: 5, trade_date: "2026-03-01" },
        { pool_count: 8, trade_date: "2026-04-01" },
        { pool_count: 13, trade_date: "2026-05-01" },
        { pool_count: 21, trade_date: "2026-06-23" },
      ],
    },
  }
}
